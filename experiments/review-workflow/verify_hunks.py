#!/usr/bin/env python3
"""Check the portable Rust diff against a prepared frozen review variant."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--example", type=Path, default=Path("target/debug/examples/text_diff"))
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    oracle = json.loads((here / "fixtures/oracle.json").read_text())
    manifest = json.loads((args.fixture / "manifest.json").read_text())
    subprocess.run([sys.executable, str(here / "verify.py"), "--fixture", str(args.fixture)], check=True)
    output = args.output.resolve()
    workspace = (args.fixture / "workspace").resolve()
    if output == workspace or workspace in output.parents:
        raise ValueError("output must be outside the fixture workspace")
    output.mkdir(parents=True, exist_ok=False)
    env = {key: value for key, value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null", GIT_OPTIONAL_LOCKS="0")
    path = oracle["source_path"]
    pin = oracle["base_revision"]
    base = subprocess.check_output(["git", "-C", str(workspace), "show", f"{pin}:{path}"], env=env)
    index = subprocess.check_output(["git", "-C", str(workspace), "show", f":{path}"], env=env)
    (output / "base.rs").write_bytes(base)
    (output / "index.rs").write_bytes(index)

    def run(logical, before, after, expected):
        result = json.loads(subprocess.check_output([str(args.example.resolve()), logical, str(before), str(after)], text=True))
        headers = [hunk["header"] for hunk in result["hunks"]]
        if headers != expected:
            raise ValueError(f"{logical}: expected {expected}, got {headers}")
        return result

    working = run(path, output / "base.rs", workspace / path,
                  oracle["variants"][manifest["variant"]]["base_to_working_hunks"])
    staged = run(path, output / "base.rs", output / "index.rs", ["@@ -166,7 +166,7 @@"])
    added = run(oracle["untracked_path"], "-", workspace / oracle["untracked_path"], ["@@ -0,0 +1,5 @@"])
    if added["base_present"] or not added["working_present"]:
        raise ValueError("untracked document lost its absent/present distinction")
    report = {"schema": 1, "variant": manifest["variant"], "base_revision": pin,
              "oracle_sha256": hashlib.sha256((here / "fixtures/oracle.json").read_bytes()).hexdigest(),
              "verified": True, "working": working, "staged": staged, "added_document": added,
              "scope": "Supplied immutable bytes; no native Git acquisition or review UI exercised."}
    (output / "hunks.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{manifest['variant']}: working={len(working['hunks'])}, staged=1, added document=1; frozen coordinates match")


if __name__ == "__main__":
    main()
