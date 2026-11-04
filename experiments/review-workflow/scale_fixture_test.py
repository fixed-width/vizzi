import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("scale_fixture", Path(__file__).with_name("scale_fixture.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class FixtureTests(unittest.TestCase):
    def test_copy_keeps_full_tree_index_original_commit_and_mixed_probe_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            parent = Path(directory)
            source = parent / "source"
            source.mkdir()
            env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
            env.update(GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", GIT_OPTIONAL_LOCKS="0")

            def git(*args):
                return subprocess.check_output(["git", "-C", str(source), "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "-c", "core.hooksPath=/dev/null", *args], env=env)

            git("init", "--quiet", "--initial-branch=main", "--template=")
            (source / "focus").mkdir()
            (source / "focus/a.py").write_bytes(b"def original(): pass\n")
            (source / "focus/b.cpp").write_bytes(b"void original() {}\n")
            (source / "outside.rs").write_bytes(b"fn outside() {}\n")
            git("add", ".")
            git("commit", "--quiet", "-m", "fixture")
            pin = git("rev-parse", "HEAD").decode().strip()
            (source / "focus/a.py").write_bytes(b"def private_change(): pass\n")
            (source / "outside.rs").write_bytes(b"fn private_change() {}\n")
            before = git("status", "--porcelain=v1", "-z")
            profile = {"name": "mixed", "source": str(source), "revision": pin, "scope": "focus", "files": ["focus/a.py", "focus/b.cpp"], "source_arguments": []}
            result = module.prepare(profile, parent / "output", "probe-one")
            workspace = parent / "output/workspace"
            self.assertEqual(result["revision"], pin)
            self.assertEqual(result["index_paths"], 3)
            self.assertEqual(result["scope_paths"], 2)
            self.assertEqual((workspace / "outside.rs").read_bytes(), b"fn outside() {}\n")
            self.assertTrue((workspace / "focus/a.py").read_bytes().startswith(b"# vizzi-review-scale probe-one initial\n"))
            self.assertTrue((workspace / "focus/b.cpp").read_bytes().startswith(b"// vizzi-review-scale probe-one initial\n"))
            self.assertEqual((parent / "output/baseline/0").read_bytes(), b"def original(): pass\n")
            self.assertEqual(git("status", "--porcelain=v1", "-z"), before)
            self.assertEqual((source / "outside.rs").read_bytes(), b"fn private_change() {}\n")
            self.assertEqual((source / "focus/a.py").read_bytes(), b"def private_change(): pass\n")
            with self.assertRaises(FileExistsError):
                module.prepare(profile, parent / "output", "probe-two")
            for scope in ["../focus", "/focus", "focus/./child", "focus//child"]:
                with self.assertRaises(ValueError):
                    module.prepare({**profile, "scope": scope}, parent / "bad", "probe")
            with self.assertRaises(ValueError):
                module.prepare({**profile, "files": ["outside.rs"]}, parent / "bad", "probe")
            with self.assertRaises(ValueError):
                module.prepare(profile, source / "inside", "probe")
            self.assertFalse((source / "inside").exists())


if __name__ == "__main__":
    unittest.main()
