#!/usr/bin/env python3
"""Observe admission boundaries with deterministic Git fixtures, separately from real workloads."""

import argparse
import json
from pathlib import Path
import subprocess
import time

from measure import Host, binding, check_window, frozen, git, history_pins, summarise


def commit(root):
    git(root, "add", ".")
    git(root, "-c", "user.name=Qualification", "-c", "user.email=qualification@example.invalid",
        "-c", "commit.gpgsign=false", "commit", "-qm", "fixture")


def make_fixture(root):
    root.mkdir()
    git(root, "init", "-q")
    for phase in range(2):
        for name, size in [("exact.rs", 2 * 1024 * 1024), ("oversized.rs", 2 * 1024 * 1024 + 1)]:
            prefix = f"fn target() {{ let phase = {phase}; helper(); }}\nfn helper() {{}}\n/*"
            (root / name).write_text(prefix + "x" * (size - len(prefix) - 3) + "*/\n")
        (root / "small.rs").write_text(f"fn target() {{ let phase = {phase}; helper(); }}\nfn helper() {{}}\n")
        commit(root)
    for phase in range(2, 68):
        (root / "small.rs").write_text(f"fn target() {{ let phase = {phase}; helper(); }}\nfn helper() {{}}\n")
        commit(root)


def page(host, frame, condition, status):
    result = host.operation("history_window", condition, "/api/function-history/window", params={
        **history_pins(frame), "line_start": 1, "line_count": 2, "reflow": "true"})
    assert host.rows[-1]["status"] == status, host.rows[-1]
    return result


def pair_page(host, pair, condition, status):
    source = frozen(pair["older"], {"kind": "comparison", "token": pair["token"], "side": "older"})
    result = host.operation("comparison_window", condition, "/api/function-comparisons/window", params={
        "token": pair["token"], "side": "older", **{k: source[k] for k in ("snapshot", "file", "version")}})
    assert host.rows[-1]["status"] == status, host.rows[-1]
    return result


def exercise(host, root):
    requests = {path: binding(host, path, "target")[0] for path in ["exact.rs", "oversized.rs", "small.rs"]}
    host.operation("history", "over_file_bytes", "/api/function-history", requests["oversized.rs"])
    assert host.rows[-1]["status"] == 503 and "2 MiB" in host.rows[-1]["result"]["message"]
    host.operation("compare", "over_file_bytes", "/api/function-comparisons", {"kind": "current", "source": requests["oversized.rs"]})
    assert host.rows[-1]["status"] == 503
    for path, count, limit, label in [("exact.rs", 17, 16, "history_bytes"), ("small.rs", 25, 24, "history_sessions")]:
        first = None
        for number in range(1, count + 1):
            frame = host.operation("history", label, "/api/function-history", requests[path])
            assert frame is not None
            first = first or frame
            if number in (limit, count):
                page(host, first, label + f"_{number}", 200 if number == limit else 409)
        source = frozen(first, {"kind": "history", "token": first["token"], "index": 0})
        host.operation("calls", label + "_expired", "/api/captured-functions", {"source": source, "query": {"kind": "calls", "offset": 0}})
        assert host.rows[-1]["status"] == 409
    for path, count, limit, label in [("exact.rs", 9, 8, "pair_bytes"), ("small.rs", 13, 12, "pair_count")]:
        first = None
        for number in range(1, count + 1):
            pair = host.operation("compare", label, "/api/function-comparisons", {"kind": "current", "source": requests[path]})
            assert pair is not None
            first = first or pair
            if number in (limit, count):
                pair_page(host, first, label + f"_{number}", 200 if number == limit else 409)
        source = frozen(first["older"], {"kind": "comparison", "token": first["token"], "side": "older"})
        host.operation("calls", label + "_expired", "/api/captured-functions", {"source": source, "query": {"kind": "calls", "offset": 0}})
        assert host.rows[-1]["status"] == 409
    frame = host.operation("history", "depth", "/api/function-history", requests["small.rs"])
    for index in range(1, 63):
        assert host.operation("deeper_history", "depth", "/api/function-history/step", {"token": frame["token"], "index": index}) is not None
    host.operation("deeper_history", "depth_boundary", "/api/function-history/step", {"token": frame["token"], "index": 63})
    assert host.rows[-1]["status"] == 503 and "64 file revisions" in host.rows[-1]["result"]["message"]
    data = page(host, frame, "retained_after_boundary", 200)
    check_window(data, history_pins(frame), git(root, "show", f"{frame['commit']}:small.rs"))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/release/vizzi"))
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    root = output / "source"
    make_fixture(root)
    report = {"fixture_head": git(root, "rev-parse", "HEAD").decode().strip(), "runs": []}
    for shallow in (False, True):
        source = root
        if shallow:
            source = output / "shallow"
            subprocess.run(["git", "clone", "-q", "--depth=1", root.as_uri(), str(source)], check=True)
        command = [str(args.binary.resolve()), "--root", str(source), "--cache", str(output / f"cache-{shallow}"), "--workers", "1", "--port", "0"]
        host = Host(command, output / f"host-{shallow}.log")
        run = {"command": command, "shallow": shallow}
        report["runs"].append(run)
        try:
            host.request("/api/workspace/start", {})
            deadline = time.monotonic() + 120
            while host.request("/api/status")[1]["readiness"] != "ready":
                assert time.monotonic() < deadline
                time.sleep(.1)
            if shallow:
                request = binding(host, "small.rs", "target")[0]
                host.operation("history", "shallow", "/api/function-history", request)
                assert host.rows[-1]["status"] == 503 and "shallow" in host.rows[-1]["result"]["message"]
            else:
                exercise(host, root)
        finally:
            host.close()
            run.update(operations=host.rows, memory_samples=host.memory, summary=summarise(host.rows))
            (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
