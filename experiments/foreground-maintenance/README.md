# Measure foreground edits during cleanup

Use the pinned Linux/PyTorch qualification archives. Build the qualified host and probe
from the workspace root:

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example foreground_maintenance --bin vizzi
python3 experiments/foreground-maintenance/measure.py \
  --cache /tmp/vizzi-foreground-facts --prepare-only \
  --output /tmp/vizzi-foreground-preparation.json
```

Cache preparation runs this project's analyser over read-only archives, without invoking
analysed project tooling. It is excluded from edit latency. After all preparation, builds,
tests and mutation jobs finish, run with a new output path:

```sh
python3 experiments/foreground-maintenance/measure.py \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-foreground-measurements/observations.json
```

The driver creates marked owned source copies and restores the ordinary target:
`kernel/locking/mutex.c` or `c10/core/Device.cpp`. The host uses four parser workers,
the normal hint/coordinator/publication path, no syntax retention/input quota, native
watching disabled and an hour-long periodic interval. Initial verification must complete
before sampling. Explicit path hints isolate watcher timing and further recovery work.

Each of three pairs alternates condition order. Both conditions first stream a generated
tail into the shared spool outside project source: 4 GiB Linux / 768 MiB PyTorch. A callback
observes entry to actual `set_len` with allocation state held; no artificial delay is added.
The control waits for cleanup completion before writing; the overlap condition writes
immediately after notification. Record actual overlap flags, including missed overlap,
rather than treating a requested condition as proof of concurrency.

Every edit adds a unique declaration. The probe verifies bytes, expected declaration,
fresh facts and old pins, restores the original observation between attempts and compares
the final result with a complete clean observation. Tail preparation and independent
verification are outside edit latency; their allocations contribute to process peaks.

Raw intervals share a monotonic clock. Entry-to-readiness marks include writer acquisition
and initial reclamation gating. Engine capture/total times describe capture and remaining
analysis/merge; hint-to-entry also includes coordinator debounce and path preparation.
Observed publication includes approximately 2 ms polling uncertainty. Qualification-only
hooks/trace are absent from default builds and do not change scheduling or quotas.

The backing allowances are 8 GiB Linux / 1 GiB PyTorch with separate equal staging
allowances. Each child has a 12 GiB virtual-address-space limit, not an RSS/physical-RAM
limit. The generated tail is controlled pressure, not typical repository code. Three
samples per condition do not establish browser tails, randomised speedups, cold-cache
behaviour or baseline-laptop performance. See [results](RESULTS.md) and the
[design](../../docs/plans/foreground-maintenance.md) for interpretation.
