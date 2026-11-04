#!/usr/bin/env python3
"""Verify native Git acquisition against the frozen review input and hunk oracle."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--example",type=Path,default=Path("target/debug/examples/review_capture"))
    args=parser.parse_args()
    here=Path(__file__).resolve().parent
    oracle=json.loads((here/"fixtures/oracle.json").read_text())
    manifest=json.loads((args.fixture/"manifest.json").read_text())
    workspace=(args.fixture/"workspace").resolve()
    output=args.output.resolve()
    if output==workspace or workspace in output.parents:
        raise ValueError("output must be outside the fixture workspace")
    if output.exists():
        raise FileExistsError(output)
    def verify():
        subprocess.run([sys.executable,str(here/"verify.py"),"--fixture",str(args.fixture)],check=True,stdout=subprocess.DEVNULL)
    verify()
    def capture(path):
        return json.loads(subprocess.check_output([str(args.example.resolve()),str(workspace),oracle["base_revision"],path],text=True))
    source=capture(oracle["source_path"])
    document=capture(oracle["untracked_path"])
    expected=oracle["variants"][manifest["variant"]]["base_to_working_hunks"]
    if [hunk["header"] for hunk in source["hunks"]]!=expected:
        raise ValueError("native Rust hunks differ from frozen coordinates")
    if sorted(row["path"] for row in source["changes"])!=sorted(oracle["changed_paths"]) or source["hidden_changes"]:
        raise ValueError("native change inventory differs from frozen paths")
    if {row["path"]:row["change"] for row in source["changes"]}!={oracle["source_path"]:"Modified",oracle["untracked_path"]:"Added"}:
        raise ValueError("incorrect native path status")
    if document["base_present"] or not document["working_present"] or [h["header"] for h in document["hunks"]]!=["@@ -0,0 +1,5 @@"]:
        raise ValueError("added document lost absent/text distinction")
    if source["baseline"]!=oracle["base_revision"] or source["comparison"]!=document["comparison"]:
        raise ValueError("native capture identity drifted across unchanged input")
    verify()
    report={"schema":1,"variant":manifest["variant"],"verified":True,"oracle_sha256":hashlib.sha256((here/"fixtures/oracle.json").read_bytes()).hexdigest(),"source":source,"document":document,"fixture_reverified_after_capture":True,"scope":"Native source-only capture and portable hunks; no host admission, analysis or UI."}
    output.parent.mkdir(parents=True,exist_ok=True)
    with output.open("x") as target:
        json.dump(report,target,indent=2);target.write("\n")
    print(f"{manifest['variant']}: native inventory and {len(source['hunks'])} source hunks match; fixture unchanged")


if __name__=="__main__":
    main()
