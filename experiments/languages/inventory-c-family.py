#!/usr/bin/env python3
"""Inventory candidate suffixes in pinned source archives without classifying languages."""

import argparse
import collections
import datetime
import hashlib
import json
import os
import pathlib


SUFFIXES = {
    ".c", ".cc", ".cpp", ".cxx", ".C", ".c++", ".h", ".hh", ".hpp",
    ".hxx", ".H", ".h++", ".cu", ".cuh", ".inc", ".inl", ".ipp", ".tpp",
}


def inventory(name, source, revision):
    root = pathlib.Path(source).resolve(strict=True)
    if not root.is_dir():
        raise NotADirectoryError(root)
    paths = []
    for directory, directories, files in os.walk(root, followlinks=False, onerror=raise_error):
        directory = pathlib.Path(directory)
        directories[:] = [
            item for item in directories
            if not (directory / item).is_symlink()
            and not (directory == root and item in {".git", "target", "node_modules"})
        ]
        for filename in files:
            path = directory / filename
            if path.suffix in SUFFIXES and not path.is_symlink() and path.is_file():
                paths.append(path.relative_to(root).as_posix())
    extensions = collections.defaultdict(lambda: {"files": 0, "bytes": 0, "examples": []})
    manifest = []
    for relative in sorted(paths):
        path = root / relative
        data = path.read_bytes()
        entry = extensions[path.suffix]
        entry["files"] += 1
        entry["bytes"] += len(data)
        if len(entry["examples"]) < 3:
            entry["examples"].append(relative)
        manifest.append([relative, len(data), hashlib.sha256(data).hexdigest()])
    encoded = json.dumps(manifest, ensure_ascii=True, separators=(",", ":")).encode()
    return {"name": name, "source": str(root), "revision": revision,
            "extensions": dict(sorted(extensions.items())),
            "manifest_sha256": hashlib.sha256(encoded).hexdigest()}


def raise_error(error):
    raise error


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", nargs=3, action="append", required=True,
                        metavar=("NAME", "PATH", "REVISION"))
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--implementation-revision", help="Vizzi revision, when known")
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    report = {
        "recorded_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "base_commit": args.implementation_revision,
        "inventory_script_sha256": hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest(),
        "scope": "Candidate suffixes only; language is not established by this inventory",
        "excluded": ["symlinks", "root .git", "root target", "root node_modules"],
        "manifest_algorithm": "SHA-256 of compact ASCII JSON, sorted [relative path, byte length, SHA-256 bytes] triples",
        "corpora": [inventory(*corpus) for corpus in args.corpus],
        "limits": ["Revisions are caller-supplied archive provenance, not inferred from Git",
                   "No submodule expansion, project builds, or compiler execution",
                   "No parsing, grammar-accuracy, or performance measurement",
                   "Includes LPC and non-code textual candidates; file kinds require source policy"],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(args.output)
