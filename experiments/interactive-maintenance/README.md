# Measure watching, recovery and automatic cleanup together

Use the pinned archives and compatible fact cache from the
[foreground procedure](../foreground-maintenance/README.md). Run from the workspace root:

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example idle_maintenance --bin vizzi
python3 experiments/interactive-maintenance/measure.py \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-interactive-measurements/observations.json
```

Choose a new output path. Finish builds, tests, mutation jobs and cache preparation before
timing. The driver creates marked owned copies, restores target hashes and runs no analysed
project tooling. Original archives remain read-only. A native watcher registration failure
aborts the experiment; polling fallback does not count as watching.

The real host uses four parser workers, native watching, two-second periodic reconciliation
and automatic idle maintenance. It supplies no explicit hints for edits or restoration.
Shared backing allowances are 8 GiB Linux / 1 GiB PyTorch; separate staging allowances equal
these bounds. Generated tails are 4 GiB / 768 MiB and contain no project code.

Run three alternating pairs of edits after completed cleanup and after observed real large
truncate entry. Then run three edits immediately after sampling recovery as active. No
artificial truncate delay or manual cleanup thread is used. Inspect actual write/cleanup
overlap and publication intervals; a requested condition alone does not prove overlap.
The scheduler waits for recovery before admitting cleanup, so simultaneous three-way work
is not forced. Recovery status before a write does not establish an exact parser interval.

Each edit must match captured source, independent fresh facts and the expected declaration,
while the retained baseline source stays readable. Restore baseline identity between
samples and compare a final complete clean capture. After coordinator shutdown, wait for
asynchronous native callback release before checking zero backing length/allocation. Raw
shutdown and callback-release timings are separate diagnostics, without a hard deadline.

Do not assign unlabelled capture marks or the latest operation's measurements to the
foreground edit: recovery can also capture and publish an overlay. This mode disables the
unlabelled trace. Periodic recovery can discover edits too, so the experiment qualifies the
combined configuration rather than isolating notification latency.

Pressure setup and independent verification are outside edit timing but contribute to
process peaks. Polling adds roughly 2 ms observation uncertainty. The 12 GiB child address
space limit does not enforce RSS/physical RAM. Three warm native samples per condition do
not establish latency tails, browser performance, cold storage or baseline laptop capacity.
See [results](RESULTS.md) and the [plan](../../docs/plans/interactive-maintenance.md) for
interpretation.
