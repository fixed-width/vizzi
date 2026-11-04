"""Create a disposable scoped worktree with a real full-size Git index."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess


def prepare(profile, output, probe):
    scope = profile["scope"]
    path = PurePosixPath(scope)
    if path.is_absolute() or ".." in path.parts or scope != path.as_posix() or scope in ("", "."):
        raise ValueError("scope must be a normalized relative subtree")
    for name in profile["files"]:
        value = PurePosixPath(name)
        if value.is_absolute() or ".." in value.parts or name != value.as_posix() or not name.startswith(scope + "/"):
            raise ValueError("probe files must be literal paths in the scope")
    pin = profile["revision"]
    if not re.fullmatch(r"[0-9a-f]{40}", pin):
        raise ValueError("revision must be a pinned SHA-1 commit")
    source = Path(profile["source"]).resolve(strict=True)
    output = Path(output).resolve()
    if output == source or source in output.parents:
        raise ValueError("output must be outside the original checkout")
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update(GIT_CONFIG_GLOBAL="/dev/null", GIT_CONFIG_NOSYSTEM="1", GIT_OPTIONAL_LOCKS="0", GIT_TERMINAL_PROMPT="0")

    def git(root, *args):
        return subprocess.run(["git", "-C", str(root), "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false", "-c", "core.autocrlf=false", "-c", "core.attributesFile=/dev/null", *args], stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True, env=env).stdout

    if Path(git(source, "rev-parse", "--show-toplevel").decode().strip()).resolve() != source:
        raise ValueError("source must be a repository root")
    if git(source, "rev-parse", "--verify", pin + "^{commit}").decode().strip() != pin:
        raise ValueError("source is missing the pinned commit")
    before = git(source, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    output.mkdir(parents=True, exist_ok=False)
    root = output / "workspace"
    git(output, "clone", "--quiet", "--shared", "--no-checkout", "--template=", str(source), str(root))
    git(root, "checkout", "--quiet", "--detach", pin)
    if git(root, "status", "--porcelain=v1", "-z"):
        raise ValueError("baseline is not clean")
    baseline = output / "baseline"
    baseline.mkdir()
    files = []
    for index, name in enumerate(profile["files"]):
        data = git(root, "show", pin + ":" + name)
        target = root / name
        if target.is_symlink() or target.read_bytes() != data:
            raise ValueError("checkout bytes differ from the baseline: " + name)
        prefix = "#" if name.endswith(".py") else "//"
        marker = f"{prefix} vizzi-review-scale {probe} initial"
        working = marker.encode() + b"\n" + data
        target.write_bytes(working)
        (baseline / str(index)).write_bytes(data)
        files.append({"path": name, "baseline": "baseline/" + str(index), "prefix": prefix, "marker": marker, "bytes": len(data), "base_sha256": hashlib.sha256(data).hexdigest(), "working_sha256": hashlib.sha256(working).hexdigest()})
    note = scope + "/vizzi-review-scale-" + probe + ".md"
    (root / note).write_text("# Review measurement\n\nDisposable comment probes; no project tooling runs.\n")
    changed = git(root, "diff", "--name-only", "-z", pin).split(b"\0")[:-1]
    untracked = git(root, "ls-files", "--others", "--exclude-standard", "-z").split(b"\0")[:-1]
    expected = sorted(profile["files"] + [note])
    if sorted(name.decode() for name in changed + untracked) != expected:
        raise ValueError("prepared changes differ from expected paths")
    after = git(source, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    if after != before:
        raise ValueError("original checkout status changed during preparation")
    manifest = {"schema": 1, **profile, "source": str(source), "workspace": "workspace", "probe": probe, "files": files, "note": note,
                "index_paths": len(git(root, "ls-files", "-z").split(b"\0")) - 1,
                "scope_paths": len(git(root, "ls-tree", "-rz", pin, "--", scope).split(b"\0")) - 1,
                "expected_changes": expected, "original_status_sha256": hashlib.sha256(before).hexdigest(), "original_status_unchanged": True}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--probe", required=True)
    args = parser.parse_args()
    result = prepare(json.loads(args.profile.read_text()), args.output, args.probe)
    print(json.dumps(result, indent=2))
