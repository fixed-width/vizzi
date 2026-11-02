# Measure foreground edits during periodic recovery

This how-to repeats the browser reference workload on disposable Linux/PyTorch copies
and selects ten ordinary edits immediately after observing separate active periodic
passes. See [results](RESULTS.md), the [scheduling contract](../../docs/prototype/RECOVERY.md),
and the general [browser procedure](../browser/README.md).

Prepare pinned archives and current-context analysis caches using the existing
[qualification procedure](../qualification/README.md). Keep cache preparation separate
from measurement. The [profiles](../qualification/profiles.json) record source scope,
explicit language assignments, exclusions, and original revisions. No analyzed project
is executed or built. Use four parser workers (driver default), warm OS caches, and the
512 MiB opt-in syntax retention used in the preceding windowed run.

Create temporary profile files with an external cache and syntax retention:

```sh
python3 - <<'PY'
import json
from pathlib import Path
for profile in json.loads(Path('experiments/qualification/profiles.json').read_text()):
    profile['cache'] = '/tmp/vizzi-window-cache/' + profile['name']
    profile['source_arguments'] += ['--syntax-cache-mib', '512']
    Path('/tmp/vizzi-recovery-' + profile['name'] + '.json').write_text(json.dumps(profile))
PY
npm test -- --measure --reference-only --during-recovery --profile /tmp/vizzi-recovery-linux.json --output results/local/recovery-linux.json
npm test -- --measure --reference-only --during-recovery --profile /tmp/vizzi-recovery-pytorch.json --output results/local/recovery-pytorch.json
```

The driver builds the optimized host and copies each archive before editing. Cache
paths above must already contain compatible facts; otherwise startup includes parsing.
Each run retains three steady and three batch-refresh motion samples, picking, ten
ordinary single-file writes, one large-header opening/edit, bounded browser response
audits, and independent final clean-capture agreement. Unique UUID comments prevent
persisted reuse of a previous probe's edit. Existing reports are never overwritten.
Run the profiles sequentially without competing tests, builds, or mutation jobs.

`recoveryBeforeWrite.running` must be true, and its `completed` values increase between
samples. This is a host status observation immediately before writing, not an instrumented
proof that every filesystem write happened before verification finished. Record both
foreground attempt measurements and separate recovery phase metadata. A later recovery
can replace latest status measurements before the collector reads them; do not attribute
those counters to the causal foreground parse without supporting evidence.

The driver checks changed snapshot identity and matching visible source DOM plus two
rAF callbacks. Native notifications remain enabled; Node status polling is diagnostic
instrumentation, separate from the actual browser's SSE transport. Software rendering,
finite samples, warm caches, and the high-memory reference host limit conclusions.
Controlled held-parser tests verify actual concurrency, bounded workers, interrupted
overlay retry, silent edits, and pin preservation separately from these timings.
