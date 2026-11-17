#!/usr/bin/env python3
"""Prepare an ordinary pinned disposable checkout with bounded upstream Git history."""

import argparse
import json
from pathlib import Path
import subprocess

from measure import git


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profile", choices=["linux", "pytorch", "lpc-rs"], required=True)
    parser.add_argument("--original", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    spec = next(p for p in json.loads(Path(__file__).with_name("profiles.json").read_text()) if p["name"] == args.profile)
    original, destination = args.original.resolve(), args.destination.resolve()
    assert not destination.exists() and not destination.is_relative_to(original)
    before = {"head": git(original, "rev-parse", "HEAD").decode().strip(),
              "status": git(original, "status", "--porcelain", "--untracked-files=all").decode(),
              "shallow": git(original, "rev-parse", "--is-shallow-repository").decode().strip()}
    commands = []

    def execute(command, **kwargs):
        commands.append(command)
        subprocess.run(command, check=True, **kwargs)

    execute(["git", "clone", "--shared", "--no-checkout", str(original), str(destination)])
    git(destination, "config", "gc.auto", "0")
    if "history_depth" in spec:
        url = {"linux": "https://github.com/torvalds/linux.git", "pytorch": "https://github.com/pytorch/pytorch.git"}[args.profile]
        git(destination, "config", "remote.history.url", url)
        execute(["git", "-C", str(destination), "fetch", "--no-tags",
                 f"--depth={spec['history_depth']}", "history", spec["revision"]])
    execute(["git", "-C", str(destination), "checkout", "--detach", spec["revision"]])
    blobs, paths = set(), {}
    for workload in spec["workloads"]:
        path = workload["path"]
        revisions = git(destination, "rev-list", "--first-parent", "-n", "65", "HEAD", "--", path).decode().splitlines()
        paths[path] = revisions
        for revision in revisions:
            result = subprocess.run(["git", "-C", str(destination), "rev-parse", f"{revision}:{path}"], capture_output=True, text=True)
            if result.returncode == 0:
                blobs.add(result.stdout.strip())
    missing = git(destination, "rev-list", "--objects", "--missing=print", "HEAD")
    assert not any(row.startswith(b"?") for row in missing.splitlines()), "Incomplete Git input"
    for blob in blobs:
        git(destination, "cat-file", "-e", blob)
    after = {"head": git(original, "rev-parse", "HEAD").decode().strip(),
             "status": git(original, "status", "--porcelain", "--untracked-files=all").decode(),
             "shallow": git(original, "rev-parse", "--is-shallow-repository").decode().strip()}
    assert before == after
    record = {"profile": spec, "original": str(original), "destination": str(destination),
              "before": before, "after": after, "commands": commands, "file_revisions": paths,
              "verified_blobs": sorted(blobs)}
    with destination.with_suffix(".preparation.json").open("x") as output:
        json.dump(record, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
