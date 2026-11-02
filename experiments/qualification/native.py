#!/usr/bin/env python3
"""Measure one full-corpus native condition, with sampled memory and explicit profiles."""

import argparse
import hashlib
import json
import os
import pathlib
import platform
import selectors
import subprocess
import time


def memory(pid):
    values = {}
    try:
        for line in pathlib.Path(f"/proc/{pid}/status").read_text().splitlines():
            key = line.split(":")[0]
            if key in {"VmRSS", "VmHWM", "VmSize", "RssAnon", "RssFile"}:
                values[key] = int(line.split()[1])
    except FileNotFoundError:
        pass
    return values


def execute(command, timeout, log):
    started = time.monotonic()
    peaks, phases, samples, events = {}, {}, [], []
    phase, last_second, pending = "startup", -1, bytearray()
    with log.with_suffix(".stderr").open("x") as stderr:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=stderr)
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        while True:
            elapsed = time.monotonic() - started
            values = memory(process.pid)
            for key, value in values.items():
                peaks[key] = max(peaks.get(key, 0), value)
                bucket = phases.setdefault(phase, {})
                bucket[key] = max(bucket.get(key, 0), value)
            if int(elapsed) != last_second:
                samples.append({"seconds": elapsed, "phase": phase, **values})
                last_second = int(elapsed)
                log.with_suffix(".progress.json").write_text(json.dumps({"elapsed_s": elapsed, "phase": phase, "peaks_kib": peaks, "latest": values}) + "\n")
            for key, _ in selector.select(.05):
                chunk = os.read(key.fd, 65536)
                if not chunk:
                    selector.unregister(key.fileobj)
                    continue
                pending.extend(chunk)
                while b"\n" in pending:
                    line, _, rest = pending.partition(b"\n")
                    pending = bytearray(rest)
                    event = json.loads(line)
                    events.append({"observed_s": time.monotonic() - started, "data": event})
                    phase = {"baseline_start": "baseline", "baseline_ready": "retained_idle", "phase_start": "capture_and_queries", "parse_started": "parse_and_queries", "phase_done": "complete"}.get(event.get("event"), phase)
            if process.poll() is not None and not selector.get_map():
                break
            if elapsed > timeout:
                process.kill()
                process.wait()
                raise TimeoutError(f"qualification exceeded {timeout}s: {command}")
        selector.close()
    if pending:
        events.append({"observed_s": time.monotonic() - started, "data": json.loads(pending)})
    stderr = log.with_suffix(".stderr").read_text()
    result = {"wall_s": time.monotonic() - started, "exit_code": process.returncode,
              "peaks_kib": peaks, "phase_peaks_kib": phases, "samples_1s": samples,
              "events": events, "stderr": stderr}
    if process.returncode != 0:
        log.with_suffix(".failed.json").write_text(json.dumps(result, indent=2) + "\n")
        raise RuntimeError(f"qualification exited {process.returncode}: {stderr}")
    return result


def policy(arguments):
    result = {"c_family_roots": [], "exclusions": [], "assignments": []}
    for flag, value in zip(arguments[::2], arguments[1::2], strict=True):
        if flag == "--c-family-root":
            result["c_family_roots"].append(value)
        elif flag == "--exclude-source":
            result["exclusions"].append(value)
        elif flag == "--syntax-language":
            scope, language = value.rsplit("=", 1)
            result["assignments"].append({"scope": scope, "language": language})
        else:
            raise ValueError(flag)
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", type=pathlib.Path, required=True)
    parser.add_argument("--profile", choices=["linux", "pytorch"], required=True)
    parser.add_argument("--action", choices=["cold", "warm", "reconcile", "reparse", "cancel"], required=True)
    parser.add_argument("--cache", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=600)
    parser.add_argument("--query-file")
    parser.add_argument("--query-interval-ms", type=int, default=50)
    parser.add_argument("--cancel-after-ms", type=int, default=100)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    if args.samples < 1:
        parser.error("samples must be positive")
    profile = next(profile for profile in json.loads(args.profiles.read_text()) if profile["name"] == args.profile)
    binary = pathlib.Path("target/release/vizzi" if args.action in ["cold", "warm"] else "target/release/examples/qualify").resolve()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    runs = []
    for sample in range(args.samples):
        log = args.output.parent / f"{args.output.stem}-sample-{sample}"
        if args.action in ["cold", "warm"]:
            cache = args.cache / f"cold-{args.workers}-{sample}" if args.action == "cold" else args.cache
            if args.action == "cold" and cache.exists():
                raise FileExistsError(cache)
            command = [str(binary), "--root", profile["source"], "--cache", str(cache), "--workers", str(args.workers), "--once", *profile["source_arguments"]]
        else:
            phase_cache = args.output.parent / f"{args.output.stem}-phase-cache-{sample}"
            if phase_cache.exists():
                raise FileExistsError(phase_cache)
            config = {"root": profile["source"], "cache": str(args.cache), "phase_cache": str(phase_cache), "workers": args.workers,
                      "policy": policy(profile["source_arguments"]), "mode": args.action, "context_revision": "full-corpus-qualification-v1",
                      "query_file": args.query_file or ("kernel/locking/mutex.c" if args.profile == "linux" else "c10/core/Device.cpp"),
                      "cancel_after_ms": args.cancel_after_ms, "query_interval_ms": args.query_interval_ms}
            config_path = log.with_suffix(".config.json")
            with config_path.open("x") as config_file:
                json.dump(config, config_file, indent=2)
            command = [str(binary), str(config_path)]
        print(f"{args.profile} {args.action} workers={args.workers} sample={sample+1}", flush=True)
        run = execute(command, args.timeout, log)
        run["command"] = command
        runs.append(run)
        last = run["events"][-1]["data"]
        if args.action in ["cold", "warm"]:
            run["status"] = last
        else:
            run["report"] = last["report"]
            assert run["report"]["pinned_source_matches"]
            if args.action == "reconcile":
                assert run["report"]["refresh"]["parsed"] == 0
            elif args.action == "cancel":
                assert run["report"]["cancel_requested"] and run["report"]["refresh"] is None
                assert run["report"]["final"]["snapshot"] == run["report"]["baseline"]["snapshot"]
            else:
                assert run["report"]["final"]["generation"] == 2
        print(f"  wall={run['wall_s']:.3f}s sampled peak RSS={run['peaks_kib'].get('VmRSS',0)/1048576:.3f}GiB", flush=True)
    report = {"profile": profile, "action": args.action, "workers": args.workers, "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "platform": platform.platform(), "cpu_affinity": sorted(os.sched_getaffinity(0)), "meminfo": pathlib.Path("/proc/meminfo").read_text(),
              "runs": runs, "limits": ["Warm OS caches; cold means an empty analysis cache", "Memory sampled every approximately 50ms; per-phase peaks are sampled lower bounds; VmHWM is process lifetime",
              "Controlled retained-snapshot example omits native watcher scheduling and HTTP transport; queries serialize a bounded core view while indexing",
              "Context revision forces real reparsing without changing source or running project tooling; source remains read-only", "This host is not a 16GB baseline laptop"]}
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(args.output, flush=True)
