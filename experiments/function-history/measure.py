#!/usr/bin/env python3
"""Measure production HTTP history operations; inspected projects are never executed."""

import argparse
import collections
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import signal
import statistics
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.parse
import urllib.request


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], timeout=120)


def summarise(rows):
    groups = {}
    for row in rows:
        key = row["operation"] + "/" + row["condition"]
        group = groups.setdefault(key, {"times": [], "refusals": collections.Counter()})
        if row["status"] == 200:
            group["times"].append(row["ms"])
        else:
            group["refusals"][str(row["status"])] += 1
    return {key: {"successes": len(g["times"]), "refusals": dict(g["refusals"]),
                  "median_ms": statistics.median(g["times"]) if g["times"] else None,
                  "max_ms": max(g["times"], default=None)} for key, g in groups.items()}


def check_window(page, pins, expected):
    """Compare literal fragments to a separate Git blob, including UTF-8 byte columns."""
    for field in ("snapshot", "file", "version"):
        assert page[field] == pins[field], (field, page[field], pins[field])
    lines = expected.split(b"\n")
    for row in page["lines"]:
        line = lines[row["number"] - 1]
        ending = "\n" if row["number"] < len(lines) else ""
        if ending and line.endswith(b"\r"):
            line, ending = line[:-1], "\r\n"
        text = row["text"].encode("utf-8")
        assert line[row["column"]:row["column"] + len(text)] == text, row
        if "ending" in row:
            assert row["ending"] == ("" if row["right"] else ending), row


def memory(pid):
    try:
        return {parts[0][:-1]: int(parts[1])
                for line in Path(f"/proc/{pid}/status").read_text().splitlines()
                if (parts := line.split())[0] in {"VmRSS:", "VmHWM:", "RssAnon:", "RssFile:"}}
    except FileNotFoundError:
        return {}


class Host:
    def __init__(self, command, log):
        self.log = log.open("x")
        self.process = subprocess.Popen(command, stdout=subprocess.DEVNULL, stderr=self.log)
        self.epoch = time.monotonic()
        self.phase = "startup"
        self.workload = None
        self.rows, self.memory, self.probes = [], [], []
        self.stop = threading.Event()
        self.pin = None
        self.reader = threading.Thread(target=self.sample, daemon=True)
        self.reader.start()
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            match = re.search(r"Vizzi: (http://127\.0\.0\.1:\d+)", log.read_text())
            if match:
                self.url = match[1]
                return
            if self.process.poll() is not None:
                break
            time.sleep(.05)
        self.close()
        raise RuntimeError(f"Host failed to listen: {log.read_text()}")

    def sample(self):
        while not self.stop.is_set():
            self.memory.append({"s": time.monotonic() - self.epoch,
                                "phase": self.phase, "workload": self.workload, **memory(self.process.pid)})
            self.stop.wait(.02)

    def request(self, route, body=None, params=None):
        if params:
            route += "?" + urllib.parse.urlencode(params)
        data = None if body is None else json.dumps(body).encode()
        request = urllib.request.Request(self.url + route, data=data,
                                         headers={"Content-Type": "application/json"})
        started = time.monotonic()
        try:
            response = urllib.request.urlopen(request, timeout=180)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            payload = response.read()
            elapsed = (time.monotonic() - started) * 1000
            try:
                result = json.loads(payload)
            except ValueError:
                result = {"message": payload.decode(errors="replace")}
            return response.code, result, elapsed, len(payload)

    def operation(self, name, condition, route, body=None, params=None):
        self.phase = name + "/" + condition
        started = time.monotonic() - self.epoch
        status, result, elapsed, size = self.request(route, body, params)
        self.rows.append({"operation": name, "condition": condition, "s": started,
                          "workload": self.workload,
                          "ms": elapsed, "status": status, "response_bytes": size,
                          "route": route, "request": body if body is not None else params,
                          "memory_kib": memory(self.process.pid),
                          "result": result if status != 200 else compact(result)})
        return result if status == 200 else None

    def background(self):
        while not self.stop.is_set():
            phase = self.phase
            for route, params in [("/api/status", None), ("/api/window", self.pin)]:
                if params is None and route.endswith("window"):
                    continue
                status, data, ms, _ = self.request(route, params=params)
                same = data.get("snapshot") == self.pin["snapshot"] if self.pin else None
                self.probes.append({"s": time.monotonic() - self.epoch, "phase": phase,
                                    "route": route, "status": status, "ms": ms,
                                    "same_snapshot": same})
            self.stop.wait(.1)

    def close(self):
        self.stop.set()
        if hasattr(self, "reader"):
            self.reader.join(timeout=2)
        if hasattr(self, "probe"):
            self.probe.join(timeout=185)
        self.process.send_signal(signal.SIGINT)
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.log.close()


def compact(value):
    """Retain identities/coverage, but hash large literal response projections."""
    if isinstance(value, list):
        return [compact(v) for v in value]
    if not isinstance(value, dict):
        return value
    window = isinstance(value.get("lines"), list) and "line_start" in value
    result = {k: compact(v) for k, v in value.items()
              if k != "origin" and not (k == "lines" and window)}
    if window:
        rows = value["lines"]
        result["line_numbers"] = [r["number"] for r in rows]
        result["lines_sha256"] = hashlib.sha256(json.dumps(rows, sort_keys=True).encode()).hexdigest()
    return result


def binding(host, path, name, scope=None):
    code, view, _, _ = host.request("/api/view", params={"projection": "summary", "filter": path})
    assert code == 200, view
    tile = next(f for f in view["files"] if f["path"] == path)
    source = view["source"]
    if source["path"] != path:
        code, view, _, _ = host.request("/api/view", params={"projection": "summary", "file": tile["id"]})
        assert code == 200, view
        source = view["source"]
    assert source["path"] == path, source
    pins = {"snapshot": view["snapshot"], "file": tile["id"], "version": source["version"]}
    rows, offset = [], 0
    while True:
        code, page, _, _ = host.request("/api/window", params={**pins, "outline_start": offset,
                                        "outline_count": 200, "line_count": 1})
        assert code == 200, page
        rows.extend(page["outline"])
        offset += len(page["outline"])
        if offset >= page["outline_total"]:
            break
        assert page["outline"], page
    matches = [r for r in rows if r["name"] == name and r["kind"] == "function"
               and (scope is None or r["scope"] == scope)]
    assert len(matches) == 1, (path, name, matches)
    entry = matches[0]
    return {**pins, "tracking": entry["id"]}, entry, tile


def frozen(frame, owner):
    s = frame["symbol"]["source"]
    return {"owner": owner, "snapshot": frame["snapshot"], "file": s["file"],
            "version": s["version"], "symbol": frame["symbol"]["id"]}


def history_pins(frame):
    s = frame["symbol"]["source"]
    return {"token": frame["token"], "snapshot": frame["snapshot"],
            "file": s["file"], "version": s["version"]}


def calls(host, frame, owner, condition, expected, hops):
    source = frozen(frame, owner)
    host.operation("actions", condition, "/api/captured-functions", {"source": source, "query": {
        "kind": "actions", "line_start": frame["symbol"]["source"]["line"], "line_count": 96, "offset": 0}})
    followed = set()
    for hop in range(hops):
        offset, candidates = 0, []
        while True:
            page = host.operation("calls", condition, "/api/captured-functions",
                                  {"source": source, "query": {"kind": "calls", "offset": offset}})
            if page is None:
                return
            candidates.extend(page["items"])
            next_offset = page["next"]
            if next_offset is None:
                break
            assert next_offset > offset
            offset = next_offset
        # Record unresolved and ambiguous definitions as well as a traversable call.
        chosen = next((c for c in candidates if c["candidate_count"] and c["id"] not in followed), None)
        unresolved = next((c for c in candidates if not c["candidate_count"]), None)
        for call in [c for c in (unresolved, chosen) if c]:
            definition = host.operation("definition", condition, "/api/captured-functions",
                {"source": source, "query": {"kind": "definition", "call": call["id"]}})
            if call is not chosen or definition is None:
                continue
            target = definition["invocation"]["candidates"][0]
            opened = host.operation("open", condition, "/api/captured-functions", {"source": source,
                "query": {"kind": "open", "call": call["id"], "target": target["id"]}})
            if opened is None:
                return
            check_window(opened["frame"]["window"], source, expected)
            assert opened["frame"]["captured"]["owner"] == owner
            followed.add(call["id"])
            source = frozen(opened["frame"], owner)
        if chosen is None:
            break


def workload(host, root, spec, samples, depth, capacity):
    path, name = spec["path"], spec["name"]
    host.workload = path + "::" + name
    request, entry, tile = binding(host, path, name, spec.get("scope"))
    host.pin = {k: request[k] for k in ("snapshot", "file", "version")}
    host.pin.update(line_start=entry["start_line"], line_count=32, outline_count=1, reflow="true")
    if not hasattr(host, "probe"):
        host.probe = threading.Thread(target=host.background, daemon=True)
        host.probe.start()
    inventory = host.operation("inventory", "coverage", "/api/structure", {"snapshot": request["snapshot"],
        "query": {"kind": "inventory", "area": path, "search": name, "offset": 0, "functions_only": True}})
    evidence = {"workload": spec, "request": request, "entry": entry, "tile": tile,
                "file_revisions": git(root, "rev-list", "--first-parent", "-n", "65", "HEAD", "--", path).decode().splitlines(),
                "inventory": inventory}
    frames, pairs = [], []
    for sample in range(samples):
        condition = "first" if sample == 0 else "warm"
        frame = host.operation("history", condition, "/api/function-history", request)
        if frame:
            expected = git(root, "show", f"{frame['commit']}:{path}")
            check_window(frame["window"], history_pins(frame), expected)
            frames.append(frame)
            host.operation("revisit", condition, "/api/function-history/step", {"token": frame["token"], "index": 0})
            pins = history_pins(frame)
            start = frame["symbol"]["source"]["line"]
            end = frame["symbol"]["source"]["end_line"]
            for line in sorted({start, min(start + 96, end), max(start, end - 31)}):
                page = host.operation("history_window", condition, "/api/function-history/window", params={
                    **pins, "line_start": line, "line_count": 96, "outline_count": 1, "reflow": "true"})
                if page:
                    check_window(page, pins, expected)
            calls(host, frame, {"kind": "history", "token": frame["token"], "index": 0}, condition, expected, 3)
        pair = host.operation("compare", condition, "/api/function-comparisons", {"kind": "current", "source": request})
        if pair:
            pairs.append(pair)
            for side in ("older", "newer"):
                part = pair[side]
                source = frozen(part, {"kind": "comparison", "token": pair["token"], "side": side})
                label = part["label"]
                data = git(root, "show", f"{label}:{path}") if side == "older" else (root / path).read_bytes()
                check_window(part["window"], source, data)
                page = host.operation("comparison_window", condition, "/api/function-comparisons/window", params={
                    "token": pair["token"], "side": side, **{k: source[k] for k in ("snapshot", "file", "version")},
                    "line_start": part["symbol"]["source"]["line"], "line_count": 96, "reflow": "true"})
                if page:
                    check_window(page, source, data)
                calls(host, part, source["owner"], condition, data, 1)
    if frames:
        frame = frames[-1]
        for index in range(1, depth + 1):
            older = host.operation("deeper_history", "discovery", "/api/function-history/step",
                                   {"token": frame["token"], "index": index})
            if older is None:
                break
            check_window(older["window"], history_pins(older), git(root, "show", f"{older['commit']}:{path}"))
        for index in (0, 1):
            host.operation("revisit", "after_discovery", "/api/function-history/step", {"token": frame["token"], "index": index})
        page = host.operation("history_window", "reacquire", "/api/function-history/window", params={
            **history_pins(frame), "line_start": frame["symbol"]["source"]["line"], "line_count": 96, "reflow": "true"})
        if page:
            check_window(page, history_pins(frame), git(root, "show", f"{frame['commit']}:{path}"))
    if capacity and frames and pairs:
        for _ in range(25):
            host.operation("history", "admission", "/api/function-history", request)
        host.operation("history_window", "expired", "/api/function-history/window", params=history_pins(frames[-1]))
        assert host.rows[-1]["status"] == 409, host.rows[-1]
        calls(host, frames[-1], {"kind": "history", "token": frames[-1]["token"], "index": 0},
              "expired", b"", 1)
        for _ in range(13):
            host.operation("compare", "admission", "/api/function-comparisons", {"kind": "current", "source": request})
        source = frozen(pairs[-1]["older"], {"kind": "comparison", "token": pairs[-1]["token"], "side": "older"})
        host.operation("comparison_window", "expired", "/api/function-comparisons/window", params={
            "token": pairs[-1]["token"], "side": "older", **{k: source[k] for k in ("snapshot", "file", "version")}})
        host.operation("calls", "expired", "/api/captured-functions", {"source": source, "query": {"kind": "calls", "offset": 0}})
        assert host.rows[-1]["status"] == 409, host.rows[-1]
    return evidence


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--profiles", type=Path, default=Path(__file__).with_name("profiles.json"))
    parser.add_argument("--profile", required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--binary", type=Path, default=Path("target/release/vizzi"))
    parser.add_argument("--cache", type=Path, help="Reuse an existing external analysis cache; recorded as seeded")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--depth", type=int, default=64)
    parser.add_argument("--capacity", action="store_true")
    args = parser.parse_args()
    profile = next(p for p in json.loads(args.profiles.read_text()) if p["name"] == args.profile)
    root, binary = args.root.resolve(), args.binary.resolve()
    assert not args.output.resolve().is_relative_to(root), "Output/cache must be outside inspected source"
    assert git(root, "rev-parse", "HEAD").decode().strip() == profile["revision"]
    remotes = subprocess.run(["git", "-C", str(root), "config", "--get-regexp", r"^remote\..*\.promisor$"],
                             capture_output=True, text=True, check=False)
    assert remotes.returncode in (0, 1), remotes.stderr
    assert not remotes.stdout, "History requires an ordinary repository without promisor configuration"
    before = git(root, "status", "--porcelain", "--untracked-files=all")
    assert not before, "Qualification needs a clean disposable checkout"
    args.output.mkdir(parents=True, exist_ok=False)
    cache = (args.cache or args.output / "cache").resolve()
    assert not cache.is_relative_to(root), "Cache must be outside inspected source"
    extra = profile.get("source_arguments", [])
    if "policy_profile" in profile:
        policy = json.loads(Path(profile["policy_profile"]).read_text())
        extra = next(p["source_arguments"] for p in policy if p["name"] == profile["name"])
    report = {"profile": profile, "command": sys.argv, "utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
              "host": {"uname": platform.uname()._asdict(), "cpu": subprocess.check_output(["lscpu"], text=True),
                       "meminfo": Path("/proc/meminfo").read_text(), "affinity": sorted(os.sched_getaffinity(0)),
                       "rustc": subprocess.check_output(["rustc", "-Vv"], text=True),
                       "python": sys.version, "git": git(root, "--version").decode()},
              "vizzi_revision": git(Path.cwd(), "rev-parse", "HEAD").decode().strip(),
              "collector_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "shallow": git(root, "rev-parse", "--is-shallow-repository").decode().strip(),
              "commits": int(git(root, "rev-list", "--count", "HEAD")), "runs": []}
    try:
        for run in range(args.runs):
            seeded = cache.exists()
            command = [str(binary), "--root", str(root), "--cache", str(cache),
                       "--port", "0", "--workers", str(args.workers), *extra]
            host = Host(command, args.output / f"host-{run}.log")
            record = {"command": command, "workloads": [], "index_cache": "seeded" if seeded else "empty"}
            report["runs"].append(record)
            try:
                host.phase = "initial_index"
                start = time.monotonic()
                host.request("/api/workspace/start", {})
                deadline = time.monotonic() + 1800
                while True:
                    code, status, _, _ = host.request("/api/status")
                    if code == 200 and status["readiness"] == "ready":
                        break
                    assert status["readiness"] != "failed", status
                    if time.monotonic() > deadline:
                        raise TimeoutError("Initial indexing exceeded 30 minutes")
                    time.sleep(.1)
                record["initial_index_s"] = time.monotonic() - start
                record["status"] = status
                for spec in profile["workloads"]:
                    print(f"{args.profile} run {run}: {spec}", flush=True)
                    record["workloads"].append(workload(host, root, spec, args.samples, args.depth,
                        args.capacity and run == 0 and spec == profile["workloads"][0]))
            finally:
                host.close()
                record.update(operations=host.rows, memory_samples=host.memory, live_probes=host.probes)
                record["summary"] = summarise(host.rows)
                record["exit_code"] = host.process.returncode
                (args.output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    finally:
        report["original_status_unchanged"] = before == git(root, "status", "--porcelain", "--untracked-files=all")
        report["head_unchanged"] = git(root, "rev-parse", "HEAD").decode().strip() == profile["revision"]
        (args.output / "result.json").write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
