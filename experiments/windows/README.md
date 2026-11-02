# Measure finite source and outline delivery

The [recorded results](RESULTS.md) separate payload bounds, interaction, and indexing costs.

Build the normal release host and compare explicit bulk diagnostics, summary views,
and three source/outline window positions on pinned Linux/PyTorch source:

```sh
cargo build --release -p vizzi --offline
python3 experiments/windows/measure.py \
  --cache /tmp/vizzi-window-cache --output /tmp/vizzi-window-payloads.json
```

The driver prepares current-context caches before measurement. It reads originals
without building or editing them, pins one snapshot/version per file, and compares
every returned fragment/declaration to that pin's bulk response. Existing outputs
are not overwritten. Run without competing tests, mutation jobs, builds, or benchmarks.

Each selected ordinary file and large header has three bulk requests, repeated summary
requests, and start/middle/end windows. Report raw HTTP-body byte counts separately
from stdio envelope/JSON-decoding response time. These observations qualify data bounds,
not browser presentation or a latency tail. Production watcher/recovery remains active.

For browser interaction, use the [existing procedure](../browser/README.md) with a
prepared cache and full profile. The browser uses summary/window requests automatically.
The driver records completed browser response sizes and checks that the page never
requests bulk views and that window bodies stay below the serialized limit. Cancelled
responses may have no body observation. The driver's own explicit bulk queries for
input selection and independent checks are separate from browser requests.

Source windows remain literal, UTF-8-safe fragments. Long lines and labels have explicit
continuation/truncation rather than silent data loss. See the
[window reference](../../docs/prototype/WINDOWS.md) for pinning and recovery semantics.
