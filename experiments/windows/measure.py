#!/usr/bin/env python3
"""Compare explicit bulk diagnostics with bounded pinned projections on real archives."""

import argparse
import hashlib
import json
import pathlib
import subprocess
import time
import urllib.parse


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", type=pathlib.Path, default=pathlib.Path("experiments/qualification/profiles.json"))
    parser.add_argument("--cache", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error("output already exists")
    binary = pathlib.Path("target/release/vizzi").resolve()
    results = []
    for profile in json.loads(args.profiles.read_text()):
        name = profile["name"]
        cache = args.cache.resolve() / name
        print(f"Preparing {name} current-context cache; excluded from observations", flush=True)
        command = [str(binary), "--root", profile["source"], "--cache", str(cache), "--workers", "4", *profile["source_arguments"]]
        subprocess.run(command + ["--once"], check=True, stdout=subprocess.DEVNULL)
        process = subprocess.Popen(command + ["--stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        serial = 0

        def request(parameters, endpoint="/api/view"):
            nonlocal serial
            serial += 1
            uri = endpoint + "?" + urllib.parse.urlencode(parameters)
            start = time.perf_counter()
            process.stdin.write(json.dumps({"id": serial, "uri": uri}) + "\n")
            process.stdin.flush()
            envelope = json.loads(process.stdout.readline())
            elapsed = (time.perf_counter() - start) * 1000
            assert envelope["status"] == 200, envelope
            body = envelope["body"]
            return json.loads(body), {"response_bytes": len(body.encode()), "stdio_response_ms": elapsed, "uri": uri}

        targets = ["kernel/locking/mutex.c", "drivers/gpu/drm/amd/include/asic_reg/dcn/dcn_3_2_0_sh_mask.h"] if name == "linux" else ["c10/core/Device.cpp", "third_party/perfetto/sdk/perfetto.h"]
        try:
            for target in targets:
                summary, observation = request({"projection": "summary", "filter": target})
                source = summary["source"]
                assert source["path"] == target and source["text"] is None
                assert all(not outline["declarations"] for outline in source["outlines"])
                pin = {"snapshot": summary["snapshot"], "file": source["id"]}
                summary_rows = [observation]
                bulk_rows = []
                for _ in range(3):
                    bulk, row = request({**pin, "filter": target})
                    bulk_rows.append(row)
                    _, row = request({**pin, "projection": "summary", "filter": target})
                    summary_rows.append(row)
                full_lines = bulk["source"]["text"].split("\n")
                full_lines = [line[:-1] if index < len(full_lines)-1 and line.endswith("\r") else line for index, line in enumerate(full_lines)]
                declarations = [entry for outline in bulk["source"]["outlines"] for entry in outline["declarations"]]
                windows = []
                for label, line, ordinal in [("start", 1, 0), ("middle", max(1, len(full_lines)//2), len(declarations)//2), ("end", len(full_lines), max(0, len(declarations)-1))]:
                    window, row = request({**pin, "version": source["version"], "line_start": line, "outline_start": ordinal, "cursor_index": ordinal}, "/api/window")
                    assert row["response_bytes"] < 512 * 1024
                    assert sum(len(line["text"].encode()) for line in window["lines"]) <= 32768
                    assert len(window["lines"]) <= 96 and len(window["outline"]) <= 200
                    for fragment in window["lines"]:
                        raw = full_lines[fragment["number"]-1].encode()
                        assert fragment["text"] == raw[fragment["column"]:fragment["end_column"]].decode()
                    for entry in window["outline"]:
                        full = declarations[entry["index"]]
                        assert entry["id"] == full["id"] and full["name"].startswith(entry["name"])
                        assert entry["start_line"] == full["start_line"] and entry["end_line"] == full["end_line"]
                    windows.append({"position": label, **row, "source_rows": len(window["lines"]), "outline_rows": len(window["outline"]), "matches_bulk": True})
                results.append({"profile": profile, "path": target, "version": source["version"], "lines": len(full_lines), "declarations": len(declarations), "bulk": bulk_rows, "summary": summary_rows, "windows": windows})
                print(f"{name} {target}: bulk={bulk_rows[0]['response_bytes']} bytes, max window={max(w['response_bytes'] for w in windows)} bytes", flush=True)
        finally:
            process.stdin.close()
            process.wait(timeout=60)
            assert process.returncode == 0
    report = {"schema": 1, "base_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(), "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "results": results,
              "limits": ["Original archives are read-only and never built", "Warm OS and prepared analysis caches; preparation excluded", "Bulk/summary comparisons share one pinned snapshot and document version", "Stdio timing includes JSON envelopes and Python decoding, not HTTP/browser presentation", "Production watcher/recovery remains active; three repeated requests and three positions do not establish a latency tail"]}
    with args.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")


if __name__ == "__main__":
    main()
