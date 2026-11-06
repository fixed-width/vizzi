import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

from weight_edit_audit import compare_tensors, safetensors_records


def record(payload, shape=None, dtype="BF16"):
    return {
        "shape": shape or [2],
        "dtype": dtype,
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


class WeightEditAuditTests(unittest.TestCase):
    def setUp(self):
        self.base = {"a": record(b"abcd"), "b": record(b"efgh"), "other": record(b"ijkl")}
        self.edited = {**self.base, "a": record(b"abce"), "b": record(b"efgi")}

    def test_accepts_exact_declared_changes_and_preserves_other_tensors(self):
        result = compare_tensors(self.base, self.edited, {"a", "b"})
        self.assertEqual(result["changed"], ["a", "b"])
        self.assertEqual(result["unchanged"], ["other"])

    def test_rejects_unrelated_change(self):
        self.edited["other"] = record(b"ijkm")
        with self.assertRaisesRegex(ValueError, "changed tensors"):
            compare_tensors(self.base, self.edited, {"a", "b"})

    def test_rejects_edit_lost_during_conversion(self):
        with self.assertRaisesRegex(ValueError, "changed tensors"):
            compare_tensors(self.base, self.base, {"a", "b"})

    def test_rejects_missing_tensor(self):
        self.edited.pop("other")
        with self.assertRaisesRegex(ValueError, "tensor names"):
            compare_tensors(self.base, self.edited, {"a", "b"})

    def test_rejects_shape_type_or_size_changes_even_on_edited_tensor(self):
        for field, value in [("shape", [1, 2]), ("dtype", "F16"), ("bytes", 8)]:
            with self.subTest(field=field):
                changed = {**self.edited, "a": {**self.edited["a"], field: value}}
                with self.assertRaisesRegex(ValueError, "tensor layout"):
                    compare_tensors(self.base, changed, {"a", "b"})

    def test_reads_payloads_without_hashing_header_or_offsets(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "fixture.safetensors"
            header = json.dumps({"__metadata__": {"format": "pt"}, "a": {"dtype": "BF16", "shape": [2], "data_offsets": [0, 4]}, "b": {"dtype": "F32", "shape": [1], "data_offsets": [4, 8]}}).encode()
            path.write_bytes(struct.pack("<Q", len(header)) + header + b"abcdefgh")
            self.assertEqual(safetensors_records(path), {"a": record(b"abcd"), "b": record(b"efgh", [1], "F32")})

    def test_rejects_truncated_payload(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "truncated.safetensors"
            header = json.dumps({"a": {"dtype": "BF16", "shape": [2], "data_offsets": [0, 4]}}).encode()
            path.write_bytes(struct.pack("<Q", len(header)) + header + b"ab")
            with self.assertRaisesRegex(ValueError, "truncated"):
                safetensors_records(path)

    def test_gguf_cli_requires_exact_changes_and_identical_metadata(self):
        import numpy as np
        from gguf import GGUFWriter

        def write(path, changes=(), template="same"):
            writer = GGUFWriter(path, "qwen35")
            writer.add_chat_template(template)
            for name in ("blk.15.ffn_down.weight", "blk.19.ffn_down.weight", "other"):
                writer.add_tensor(name, np.full((2, 4), 2 if name in changes else 1, dtype=np.float32))
            writer.write_header_to_file()
            writer.write_kv_data_to_file()
            writer.write_tensors_to_file()
            writer.close()

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            write(root / "base.gguf")
            manifest = root / "manifest.json"
            manifest.write_text('{"tensors": []}')
            expected = ("blk.15.ffn_down.weight", "blk.19.ffn_down.weight")
            for name, changes, template, error in [
                ("valid", expected, "same", None),
                ("unrelated", (*expected, "other"), "same", "changed tensors"),
                ("lost", (), "same", "changed tensors"),
                ("metadata", expected, "different", "GGUF metadata"),
            ]:
                with self.subTest(name=name):
                    path = root / (name + ".gguf")
                    write(path, changes, template)
                    output = root / (name + ".json")
                    result = subprocess.run([sys.executable, str(Path(__file__).with_name("weight_edit_audit.py")), "gguf", str(root / "base.gguf"), str(path), "--manifest", str(manifest), "--output", str(output)], capture_output=True, text=True)
                    if error:
                        self.assertNotEqual(result.returncode, 0)
                        self.assertIn(error, result.stderr)
                        self.assertFalse(output.exists())
                    else:
                        self.assertEqual(result.returncode, 0, result.stderr)
                        self.assertEqual(json.loads(output.read_text())["changed"], list(expected))


if __name__ == "__main__":
    unittest.main()
