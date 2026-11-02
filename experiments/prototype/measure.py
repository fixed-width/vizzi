#!/usr/bin/env python3
"""Measure the native application through its public diagnostic transport.

All edits affect a temporary copy. This measures receipt of a coherent response,
not browser presentation or rendering. Run without competing benchmark work.
"""

import argparse
import hashlib
import json
import pathlib
import platform
import re
import selectors
import shutil
import subprocess
import tempfile
import time
import urllib.parse


def milliseconds(start):
    return (time.perf_counter() - start) * 1000


def once(binary, root, cache, workers, source_arguments=()):
    started = time.perf_counter()
    result = subprocess.run(
        ["/usr/bin/time", "-f", "VIZZI_RSS_KIB=%M", str(binary), "--root", str(root),
         "--cache", str(cache), "--workers", str(workers), "--once", *source_arguments],
        capture_output=True, text=True, check=True, timeout=60,
    )
    return {"process_wall_ms": milliseconds(started),
            "peak_rss_kib": int(re.search(r"VIZZI_RSS_KIB=(\d+)", result.stderr)[1]),
            "status": json.loads(result.stdout)}


class Diagnostic:
    def __init__(self, binary, root, cache, workers, source_arguments=()):
        self.process = subprocess.Popen(
            [str(binary), "--root", str(root), "--cache", str(cache),
             "--workers", str(workers), "--stdio", *source_arguments],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.buffer = b""
        self.sequence = 0

    def request(self, uri):
        self.sequence += 1
        started = time.perf_counter()
        self.process.stdin.write((json.dumps({"id": self.sequence, "uri": uri}) + "\n").encode())
        self.process.stdin.flush()
        deadline = time.monotonic() + 30
        while b"\n" not in self.buffer:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not self.selector.select(remaining):
                raise TimeoutError("diagnostic response timed out")
            chunk = self.process.stdout.read1(65536)
            if not chunk:
                raise RuntimeError("diagnostic host exited: " + self.process.stderr.read().decode())
            self.buffer += chunk
        line, self.buffer = self.buffer.split(b"\n", 1)
        response = json.loads(line)
        assert response["id"] == self.sequence and response["status"] == 200, response
        return json.loads(response["body"]), milliseconds(started)

    def wait(self, predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            status, _ = self.request("/api/status")
            if predicate(status):
                return status
            time.sleep(0.003)
        raise TimeoutError("workspace publication timed out")

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.selector.close()
        assert self.process.returncode == 0, self.process.stderr.read().decode()


def measure(args):
    binary = args.binary.resolve()
    source_arguments = []
    for flag, values in [("--c-family-root", args.c_family_root), ("--syntax-language", args.syntax_language), ("--exclude-source", args.exclude_source)]:
        for value in values:
            source_arguments.extend([flag, value])
    with tempfile.TemporaryDirectory(prefix="vizzi-workspace-") as temporary:
        base = pathlib.Path(temporary)
        root, cache = base / "source", base / "cache"
        shutil.copytree(args.source, root, symlinks=True)
        cold, warm = [], []
        for index in range(3):
            isolated_cache = base / f"cold-cache-{index}"
            cold.append(once(binary, root, isolated_cache, args.workers, source_arguments))
        shutil.copytree(base / "cold-cache-0", cache)
        for _ in range(3):
            warm.append(once(binary, root, cache, args.workers, source_arguments))
        assert all(run["status"]["measurements"]["parsed"] == 0 for run in warm)
        assert len({run["status"]["snapshot"] for run in cold + warm}) == 1
        session = Diagnostic(binary, root, cache, args.workers, source_arguments)
        try:
            first, first_ms = session.request("/api/view")
            inventory = {"complete": len(first["files"]) == first["total"], "files": [{"path": file["path"], "bytes": file["bytes"], "coverage": file["coverage"], "classification": file["classification"]} for file in first["files"]]}
            if args.select_file:
                selected, _ = session.request("/api/view?" + urllib.parse.urlencode({"filter": args.select_file}))
                file = next(file for file in selected["files"] if file["path"] == args.select_file)
                first, first_ms = session.request("/api/view?" + urllib.parse.urlencode({"file": file["id"]}))
            assert first["schema"] == 2
            file_id = first["selection"]["file"]
            source = first["source"]
            assert source["outlines"], "select a file with assigned analysis"
            declarations = [fact for outline in source["outlines"] for fact in outline["declarations"]]
            entity_id = declarations[0]["id"] if declarations else None
            uri = "/api/view?" + urllib.parse.urlencode({"file": file_id, **({"entity": entity_id} if entity_id else {})})
            settled = session.wait(lambda status: not status["pending"] and status["measurements"]["reused"] == first["total"])
            no_op = settled["measurements"]
            pinned_uri = "/api/source?" + urllib.parse.urlencode({"snapshot": first["snapshot"], "file": file_id})
            target = root / source["path"]
            original = target.read_bytes()
            comment = "#" if source["outlines"][0]["context"]["language"] == "python" else "//"
            previous_snapshot = first["snapshot"]
            edits = []
            for index in range(3):
                started = time.perf_counter()
                target.write_bytes(f"{comment} vizzi edit probe {index}\n".encode() + original)
                status = session.wait(lambda status: status["snapshot"] != previous_snapshot and not status["pending"])
                publication_ms = milliseconds(started)
                updated, request_ms = session.request(uri)
                response_ms = milliseconds(started)
                assert updated["selection"]["file"] == file_id
                assert updated["selection"]["entity"] == entity_id
                assert updated["source"]["text"].startswith(f"{comment} vizzi edit probe {index}\n")
                assert status["measurements"]["parsed"] == 1, status
                assert status["measurements"]["read_files"] == 1, status
                clean = once(binary, root, cache, args.workers, source_arguments)
                assert clean["status"]["snapshot"] == updated["snapshot"], "incremental index differs from clean capture"
                if index == 0:
                    pinned, _ = session.request(pinned_uri)
                    assert pinned["text"] == source["text"]
                edits.append({"write_to_publication_ms": publication_ms, "write_to_response_ms": response_ms,
                              "view_request_ms": request_ms, "measurements": status["measurements"], "clean_snapshot_matches": True})
                previous_snapshot = updated["snapshot"]
            peak = re.search(r"VmHWM:\s+(\d+) kB", pathlib.Path(f"/proc/{session.process.pid}/status").read_text())
            return {"source_revision": args.revision, "source": str(args.source.resolve()),
                    "scope": "Configured raw capture; disposable copy; no analyzed project builds",
                    "source_arguments": source_arguments, "capture_policy": first["policy"],
                    "selected_classification": source["classification"], "captured_inventory": inventory,
                    "platform": platform.platform(), "workers": args.workers,
                    "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
                    "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                    "cold": cold, "warm": warm, "no_op": no_op, "edits": edits,
                    "diagnostic_initial_view_ms": first_ms, "selected_path": source["path"],
                    "selected_declaration": declarations[0]["name"] if declarations else None,
                    "analysis_context": source["outlines"][0]["context"],
                    "live_peak_rss_kib": int(peak[1]) if peak else None,
                    "limits": ["Three samples; no tail-latency claim", "First load has warm OS file cache",
                               "No browser, labels, frame times, GPU, or edit-to-visible measurement",
                               "Hinted files are reopened and hashed; full reconciliation verifies the whole configured source scope; changed files are parsed afresh"]}
        finally:
            session.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=pathlib.Path, default=pathlib.Path("target/release/vizzi"))
    parser.add_argument("--source", type=pathlib.Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--c-family-root", action="append", default=[])
    parser.add_argument("--syntax-language", action="append", default=[])
    parser.add_argument("--exclude-source", action="append", default=[])
    parser.add_argument("--select-file", help="Exact relative path of the file to edit")
    parser.add_argument("--output", type=pathlib.Path, required=True)
    arguments = parser.parse_args()
    report = measure(arguments)
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    with arguments.output.open("x") as output:
        json.dump(report, output, indent=2)
        output.write("\n")
    print(arguments.output)
