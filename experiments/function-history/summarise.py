#!/usr/bin/env python3
"""Export successful latency, refusals and sampled native resources without source text."""

import argparse
import collections
import csv
import gzip
import json
from pathlib import Path

from measure import summarise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", type=Path, nargs="+")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    result = {}
    with (args.output / "operations.csv").open("x", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=["profile", "input", "run", "workload", "operation", "condition", "status", "ms", "response_bytes"], lineterminator="\n")
        writer.writeheader()
        for path in args.inputs:
            opener = gzip.open if path.suffix == ".gz" else open
            with opener(path, "rt") as source:
                report = json.load(source)
            profile = report["profile"]["name"]
            groups = collections.defaultdict(list)
            resources = []
            for index, run in enumerate(report["runs"]):
                for row in run["operations"]:
                    groups[row["workload"]].append(row)
                    writer.writerow({"profile": profile, "input": str(path), "run": index,
                                     **{k: row[k] for k in writer.fieldnames[3:]}})
                probes = [{**r, "operation": r["route"], "condition": "coexistence"} for r in run["live_probes"]]
                samples = run["memory_samples"]
                resources.append({"input": str(path), "run": index, "initial_index_s": run["initial_index_s"],
                                  "process_peak_rss_kib": max(s.get("VmRSS", 0) for s in samples),
                                  "lifetime_hwm_kib": max(s.get("VmHWM", 0) for s in samples),
                                  "post_index_peak_rss_kib": max((s.get("VmRSS", 0) for s in samples if s["workload"]), default=None),
                                  "live_responsiveness": summarise(probes),
                                  "changed_live_pins": sum(r["same_snapshot"] is False for r in probes)})
            entry = result.setdefault(profile, {"workloads": {}, "resources": []})
            assert not entry["workloads"].keys() & groups.keys(), "Workload repeated across input files"
            entry["workloads"].update({name: summarise(rows) for name, rows in groups.items()})
            entry["resources"].extend(resources)
    (args.output / "summary.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
