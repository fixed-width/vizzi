#!/usr/bin/env python3
"""Plot recorded native qualification times against sampled process memory."""

import argparse
import json
import pathlib
import statistics

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("summary", type=pathlib.Path)
parser.add_argument("output", type=pathlib.Path)
args = parser.parse_args()
if args.output.exists():
    parser.error("output already exists")
data = json.loads(args.summary.read_text())
fig, axes = plt.subplots(1, 2, figsize=(11, 4.6))
labels = {("cold", 4): "Cold 4", ("warm", 4): "Warm 4", ("reconcile", 4): "Verify 4", ("reparse", 4): "Retained reparse 4", ("reparse", 8): "Retained reparse 8"}
colors = ["#526477", "#8d9baa", "#2677b5", "#d17a20", "#a33a3a"]
for ax, profile in zip(axes, ["linux", "pytorch"]):
    for (action, workers), color in zip(labels, colors):
        condition = next(item for item in data["conditions"] if item["profile"] == profile and item["action"] == action and item["workers"] == workers)
        xs = [row["engine_s"] for row in condition["rows"]]
        ys = [row["peak_rss_gib"] for row in condition["rows"]]
        x, y = statistics.median(xs), statistics.median(ys)
        ax.errorbar(x, y, xerr=[[x-min(xs)], [max(xs)-x]], yerr=[[y-min(ys)], [max(ys)-y]], fmt="o", color=color, capsize=3, label=labels[(action, workers)])
    ax.set_title("Linux, 66,066 files" if profile == "linux" else "PyTorch, 10,578 files")
    ax.set_xlabel("Engine elapsed time (s)")
    ax.set_ylabel("Sampled process peak RSS (GiB)")
    ax.set_xlim(left=0)
    ax.set_ylim(bottom=0)
    ax.grid(alpha=.2)
axes[1].legend(fontsize=8, loc="lower right")
fig.suptitle("Full-corpus native qualification · median and range of three runs")
fig.text(.5, .01, "Per-panel scales differ. Warm OS caches; retained probes include old snapshots and serialized view queries. No 16 GB laptop emulation.", ha="center", fontsize=8)
fig.tight_layout(rect=(0, .045, 1, .94))
fig.savefig(args.output)
args.output.write_text("\n".join(line.rstrip() for line in args.output.read_text().splitlines()) + "\n")
print(args.output)
