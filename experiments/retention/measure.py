#!/usr/bin/env python3
"""Compare disabled/bounded syntax retention on owned full-repository copies."""

import argparse
import hashlib
import json
import pathlib
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
    parser.add_argument("--samples", type=int, default=3)
    args = parser.parse_args()
    if args.output.exists() or not 1 <= args.samples <= 8:
        parser.error("output must not exist and samples must be in 1..8")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    binary = pathlib.Path("target/release/examples/retain").resolve()
    host = pathlib.Path("target/release/vizzi").resolve()
    results = []
    for profile in json.loads(args.profiles.read_text()):
        name = profile["name"]
        if args.profile not in [name, "both"]:
            continue
        cache = args.cache.resolve() / name
        print(f"Preparing {name} current-context cache; excluded from timed cases", flush=True)
        subprocess.run([str(host), "--root", profile["source"], "--cache", str(cache), "--workers", "4", "--once", *profile["source_arguments"]], check=True, stdout=subprocess.DEVNULL)
        targets = ["kernel/locking/mutex.c", "drivers/gpu/drm/amd/include/asic_reg/dcn/dcn_3_2_0_sh_mask.h"] if name == "linux" else ["c10/core/Device.cpp", "third_party/perfetto/sdk/perfetto.h"]
        with tempfile.TemporaryDirectory(prefix="vizzi-retention-owned-") as temporary:
            root = pathlib.Path(temporary) / "source"
            shutil.copytree(profile["source"], root, symlinks=True)
            owner = str(uuid.uuid4())
            (root / ".vizzi-retention-owned").write_text(owner)
            original = {path: hashlib.sha256((root / path).read_bytes()).hexdigest() for path in targets}
            for budget in [0, 512]:
                config = {"root": str(root), "cache": str(cache), "policy": policy(profile["source_arguments"]), "workers": 4, "targets": targets, "samples": args.samples,
                          "budget_mib": budget, "probe": str(uuid.uuid4()), "owner": owner}
                log = args.output.parent / f"{args.output.stem}-{name}-{budget}"
                config_path = log.with_suffix(".config.json")
                with config_path.open("x") as output:
                    json.dump(config, output, indent=2)
                print(f"{name} retention={budget} MiB; {args.samples} edits at each of three positions per file", flush=True)
                run = execute([str(binary), str(config_path)], 900, log)
                run["report"] = run["events"][-1]["data"].pop("report")
                run["events"][-1]["data"]["report_ref"] = "run.report"
                assert all(hashlib.sha256((root / path).read_bytes()).hexdigest() == digest for path, digest in original.items())
                results.append({"profile": profile, "configuration": config, "original_target_sha256": original, "run": run})
                print(f"  completed; sampled process peak including clean verification {run['peaks_kib'].get('VmRSS', 0)/1048576:.3f} GiB", flush=True)
    report = {"schema": 1, "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "host_sha256": hashlib.sha256(host.read_bytes()).hexdigest(), "results": results,
              "limits": ["Warm OS caches; current-context analysis cache prepared before timing", "One separate process per disabled/enabled condition; priming is separate from repeated edits",
                         "Unique per-condition probe inputs require real parsing, not persisted cache hits", "No watcher, HTTP, or browser timing; all cases use production engine/extraction",
                         "Process peaks include clean-capture verification; point RSS is also recorded before each verification", "Admission weight is a retention quota, not a measured heap cap or whole-process memory limit",
                         "Four shared workers; only one targeted file is changed in a case; this host is not the 16GB baseline laptop"]}
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
