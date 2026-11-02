#!/usr/bin/env python3
"""Validate cross-run identities and export compact native qualification summaries."""

import argparse
import csv
import json
import math
import pathlib


def summary(values):
    values = sorted(values)
    return {"count": len(values), **{label: values[math.ceil(len(values) * fraction) - 1] if values else None
                                   for label, fraction in [("p50", .5), ("p95", .95), ("p99", .99), ("max", 1)]}}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    conditions, csv_rows, profiles = [], [], {}
    for path in sorted(args.directory.glob("*.json")):
        if any(token in path.name for token in ["sample-", "progress", "failed"]):
            continue
        report = json.loads(path.read_text())
        if "action" not in report:
            continue
        rows = []
        for index, run in enumerate(report["runs"]):
            probe = run.get("report")
            status = probe["final"] if probe else run["status"]
            measurements = probe["refresh"] if probe else status["measurements"]
            times = [sample["ms"] for sample in probe["queries"]] if probe else []
            events = run["events"]
            at = {event["data"].get("event"): event["observed_s"] for event in events if "event" in event["data"]}
            row = {"profile": report["profile"]["name"], "action": report["action"], "workers": report["workers"], "sample": index,
                   "wall_s": run["wall_s"], "engine_s": measurements["total_ms"] / 1000 if measurements else None,
                   "phase_s": probe["phase_wall_ms"] / 1000 if probe else None,
                   "peak_rss_gib": run["peaks_kib"].get("VmRSS", 0) / 1048576,
                   "lifetime_hwm_gib": run["peaks_kib"].get("VmHWM", 0) / 1048576,
                   "query_p50_ms": summary(times)["p50"], "query_p95_ms": summary(times)["p95"], "query_p99_ms": summary(times)["p99"],
                   "query_count": len(times), "cancel_response_s": at.get("phase_done", 0) - at["cancel_requested"] if "cancel_requested" in at else None,
                   "snapshot": status["snapshot"], "files": status["measurements"]["files"], "analyzed": status["measurements"]["analyzed"],
                   "source_only": status["measurements"]["source_only"], "input_bytes": status["measurements"]["bytes"]}
            rows.append(row)
            csv_rows.append(row)
        assert len({row["snapshot"] for row in rows}) == 1, path
        profiles[report["profile"]["name"]] = report["profile"]
        conditions.append({"file": path.name, "profile": report["profile"]["name"], "action": report["action"], "workers": report["workers"],
                           "binary_sha256": report["binary_sha256"], "rows": rows,
                           "limits": report["limits"]})
    for name in {condition["profile"] for condition in conditions}:
        regular = [row["snapshot"] for condition in conditions if condition["profile"] == name and condition["action"] != "reparse" for row in condition["rows"]]
        reparse = [row["snapshot"] for condition in conditions if condition["profile"] == name and condition["action"] == "reparse" for row in condition["rows"]]
        assert len(set(regular)) == 1, name
        assert len(set(reparse)) == 1 and set(regular).isdisjoint(reparse), name
    with args.output.open("x") as output:
        json.dump({"profiles": profiles, "conditions": conditions, "identities_agree_across_samples_workers_and_cache_state": True}, output, indent=2)
        output.write("\n")
    with args.output.with_suffix(".csv").open("x", newline="") as output:
        writer = csv.DictWriter(output, fieldnames=list(csv_rows[0]), lineterminator="\n")
        writer.writeheader()
        writer.writerows(csv_rows)
    print(args.output)
