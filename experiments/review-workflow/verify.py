#!/usr/bin/env python3
"""Verify a prepared fixture against frozen bytes, inventory and hunk coordinates."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, required=True)
    args = parser.parse_args()
    fixtures = Path(__file__).resolve().parent / "fixtures"
    oracle = json.loads((fixtures / "oracle.json").read_text())
    manifest = json.loads((args.fixture / "manifest.json").read_text())
    variant = oracle["variants"][manifest["variant"]]
    workspace = args.fixture / "workspace"
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null", GIT_OPTIONAL_LOCKS="0")

    def git(*command):
        return subprocess.check_output(
            ["git", "-C", str(workspace), "-c", "core.fsmonitor=false",
             "-c", "core.excludesFile=/dev/null", *command], env=env,
        )

    def check(actual, expected, label):
        if actual != expected:
            raise ValueError(f"{label}: expected {expected!r}, got {actual!r}")

    def check_hash(data, expected, label):
        check(hashlib.sha256(data).hexdigest(), expected, label)

    pin = oracle["base_revision"]
    path = oracle["source_path"]
    check(git("rev-parse", "HEAD").decode().strip(), pin, "baseline commit")
    git("fsck", "--no-reflogs", "--no-dangling")
    status = git("status", "--porcelain=v1", "-z", "--untracked-files=all").decode().split("\0")[:-1]
    check(sorted(status), sorted(oracle["status_porcelain"]), "stage/working status")
    tracked = git("diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--name-only", "-z", pin).decode().split("\0")[:-1]
    untracked = git("ls-files", "--others", "--exclude-standard", "-z").decode().split("\0")[:-1]
    check(sorted(tracked + untracked), sorted(oracle["changed_paths"]), "changed inventory")
    base = git("show", f"{pin}:{path}")
    index = git("show", f":{path}")
    working = (workspace / path).read_bytes()
    check_hash(base, oracle["base_sha256"], "base source")
    check_hash(index, oracle["index_sha256"], "staged source")
    check_hash(working, variant["working_sha256"], "working source")
    check_hash((workspace / oracle["untracked_path"]).read_bytes(), oracle["document_sha256"], "untracked document")
    for name, expected in variant["patch_sha256"].items():
        check_hash((fixtures / manifest["variant"] / name).read_bytes(), expected, name)
    patch = git("diff", "--no-ext-diff", "--no-textconv", "--no-renames", "--no-color", "--diff-algorithm=myers", "--unified=3", pin, "--", path).decode()
    # Git may append a function heading; coordinates belong to the immutable pair.
    hunks = [line[:line.index("@@", 2) + 2] for line in patch.splitlines() if line.startswith("@@")]
    check(hunks, variant["base_to_working_hunks"], "base/working hunk coordinates")
    if b"ErrorKind::NotFound | ErrorKind::PermissionDenied" in index:
        raise ValueError("seeded regression leaked into the staged diagnostic change")
    print(json.dumps({"variant": manifest["variant"], "verified": True,
                      "changed_paths": len(tracked + untracked), "hunks": len(hunks),
                      "working_differs_from_index": working != index}, indent=2))


if __name__ == "__main__":
    main()
