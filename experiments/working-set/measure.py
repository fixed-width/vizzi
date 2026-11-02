#!/usr/bin/env python3
"""Measure retained-tree availability across serial and batched mixed working sets."""
import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
import tempfile
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from experiments.qualification.native import execute, policy


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", type=pathlib.Path, default=pathlib.Path("experiments/qualification/profiles.json"))
    parser.add_argument("--profile", choices=["linux", "pytorch", "both"], default="both")
    parser.add_argument("--cache", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("refusing to overwrite output")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binary = pathlib.Path("target/release/examples/working_set").resolve()
    profiles = json.loads(args.profiles.read_text())
    previous = pathlib.Path("experiments/recovery/results")
    results = []
    for profile in profiles:
        name = profile["name"]
        if args.profile not in ["both", name]:
            continue
        reference = json.loads((previous / f"browser-{name}.json").read_text())["results"][0]
        large = [file["path"] for file in reference["heavyFiles"]]
        small = reference["picking"]["path"]
        targets = large + [small]
        steps = [
            {"kind": "edit", "paths": [large[0]]},
            {"kind": "edit", "paths": [large[0]]},
            {"kind": "edit", "paths": [small]},
            {"kind": "edit", "paths": [large[0]]},
            {"kind": "edit", "paths": [large[1]]},
            {"kind": "edit", "paths": [large[2]]},
            {"kind": "edit", "paths": [large[0]]},
            {"kind": "edit", "paths": large},
            {"kind": "reconcile"},
            *[{"kind": "edit", "paths": [small]} for _ in range(10)],
            {"kind": "edit", "paths": [large[0]]},
        ]
        with tempfile.TemporaryDirectory(prefix="vizzi-working-set-owned-") as temporary:
            root = pathlib.Path(temporary) / "source"
            shutil.copytree(profile["source"], root, symlinks=True)
            owner = str(uuid.uuid4())
            (root / ".vizzi-working-set-owned").write_text(owner)
            original = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in targets}
            baselines = set()
            for budget in [0, 512, 1024]:
                config = {"root": str(root), "cache": str(args.cache.resolve() / name),
                          "policy": policy(profile["source_arguments"]), "workers": 4, "targets": targets,
                          "steps": steps, "budget_mib": budget, "owner": owner, "probe": str(uuid.uuid4())}
                log = args.output.parent / f"{args.output.stem}-{name}-{budget}"
                config_path = log.with_suffix(".config.json")
                with config_path.open("x") as output:
                    json.dump(config, output, indent=2)
                print(f"{name} retention={budget} MiB; serial, batched, reconcile and ten ordinary edits", flush=True)
                run = execute([str(binary), str(config_path)], 900, log)
                report = run["events"][-1]["data"].pop("report")
                run["events"][-1]["data"]["report_ref"] = "report"
                assert report["baseline_measurements"]["parsed"] == 0, "prepare compatible analysis caches first"
                baselines.add(report["baseline_snapshot"])
                assert all(hashlib.sha256((root / p).read_bytes()).hexdigest() == h for p, h in original.items())
                results.append({"profile": profile, "configuration": config, "original_target_sha256": original, "run": run, "report": report})
                print(f"  final large edit tree reuses={report['cases'][-1]['measurements']['tree_reuses']}; sampled lifetime peak={run['peaks_kib'].get('VmRSS',0)/1048576:.3f} GiB", flush=True)
            assert len(baselines) == 1, "restored baseline changed across budgets"
    result = {"schema": 1, "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "platform": platform.platform(),
              "cpu_affinity": sorted(os.sched_getaffinity(0)), "meminfo": pathlib.Path("/proc/meminfo").read_text(), "results": results,
              "limits": ["Warm OS caches and previously prepared current-context analysis cache; baseline parsing must be zero",
                         "Separate sequential process per budget; unique comments; all four files share one cache throughout a condition",
                         "Each completed step matches independent fresh analysis for changed files and a full snapshot capture",
                         "No watcher, host scheduling or browser; three-file batches have four-worker maximum concurrency",
                         "Memory samples and lifetime high water include fresh verification; per-step point RSS precedes verification",
                         "Entry inspection does not lease state or change LRU order; batch observations do not identify internal interleavings",
                         "Retention weight and per-entry allowance are not a parser-input or process-memory cap; baseline laptop unqualified"]}
    with args.output.open("x") as output:
        json.dump(result, output, indent=2)
        output.write("\n")
    print(args.output, flush=True)


if __name__ == "__main__":
    main()
