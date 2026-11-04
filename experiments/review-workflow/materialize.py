#!/usr/bin/env python3
"""Prepare one frozen review variant in a new disposable directory."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile


FIXTURES = Path(__file__).resolve().parent / "fixtures"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--variant", choices=("variant-a", "variant-b"), required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    oracle = json.loads((FIXTURES / "oracle.json").read_text())
    pin = oracle["base_revision"]
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null",
               GIT_OPTIONAL_LOCKS="0", GIT_TERMINAL_PROMPT="0")

    def git(root, *command, data=None):
        return subprocess.run(
            ["git", "-C", str(root), "-c", "core.fsmonitor=false",
             "-c", "core.hooksPath=/dev/null", "-c", "core.autocrlf=false",
             "-c", "core.attributesFile=/dev/null", *command],
            input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            check=True, env=env,
        ).stdout

    source = args.source.resolve(strict=True)
    if Path(git(source, "rev-parse", "--show-toplevel").decode().strip()).resolve() != source:
        raise ValueError("--source must name the repository root")
    output = args.output.resolve()
    if output == source or source in output.parents:
        raise ValueError("output must be outside the original checkout")
    original_status = git(source, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    commit = git(source, "cat-file", "commit", pin)
    expected_tree = git(source, "rev-parse", f"{pin}^{{tree}}").strip()
    for name, expected in oracle["variants"][args.variant]["patch_sha256"].items():
        if hashlib.sha256((FIXTURES / args.variant / name).read_bytes()).hexdigest() != expected:
            raise ValueError(f"frozen patch changed: {name}")
    output.mkdir(parents=True, exist_ok=False)
    workspace = output / "workspace"
    workspace.mkdir()
    with tempfile.TemporaryFile() as archive:
        subprocess.run(["git", "-C", str(source), "archive", "--format=tar", pin],
                       stdout=archive, check=True, env=env)
        archive.seek(0)
        with tarfile.open(fileobj=archive) as contents:
            contents.extractall(workspace, filter="data")

    template = output / "empty-template"
    template.mkdir()
    git(workspace, "init", "--quiet", "--initial-branch=review", f"--template={template}")
    git(workspace, "add", "--force", "--all")
    if git(workspace, "write-tree").strip() != expected_tree:
        raise ValueError("archive does not reproduce the pinned Git tree")
    if git(workspace, "hash-object", "-t", "commit", "-w", "--stdin", data=commit).strip().decode() != pin:
        raise ValueError("baseline commit identity changed")
    (workspace / ".git" / "shallow").write_text(pin + "\n")
    git(workspace, "update-ref", "refs/heads/review", pin)
    if git(workspace, "status", "--porcelain=v1", "-z"):
        raise ValueError("baseline is not clean")

    git(workspace, "apply", "--index", str(FIXTURES / args.variant / "staged.patch"))
    git(workspace, "apply", str(FIXTURES / args.variant / "unstaged.patch"))
    document = workspace / oracle["untracked_path"]
    if document.exists():
        raise ValueError("fixture document unexpectedly exists at baseline")
    document.parent.mkdir(parents=True, exist_ok=True)
    document.write_bytes((FIXTURES / "source-reading.md").read_bytes())
    if git(source, "status", "--porcelain=v1", "-z", "--untracked-files=all") != original_status:
        raise ValueError("original checkout status changed during preparation")
    manifest = {"schema": 1, "variant": args.variant, "base_revision": pin,
                "workspace": "workspace", "original_status_unchanged": True,
                "git_version": git(source, "--version").decode().strip()}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Prepared {args.variant}: {workspace}")


if __name__ == "__main__":
    main()
