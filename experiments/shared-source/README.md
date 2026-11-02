# Compare shared-source capture resources

The [recorded results](RESULTS.md) include both memory improvements and timing tradeoffs.

Use the existing [qualification procedure](../qualification/README.md) to compare
fresh-process cached loads (`warm`), full verification with the old snapshot retained
(`reconcile`), and full context reparse with that snapshot retained (`reparse`).

Run each condition three times on Linux and PyTorch with the unchanged pinned profiles,
four workers, warm OS caches, and no competing builds/tests/mutation jobs. Build the
normal host and the optional `qualify` example for each implementation. Prepare its
current-context cache separately before timing; model/fingerprint changes invalidate
old context entries. Use distinct output and phase-cache paths.

```sh
python3 experiments/qualification/native.py \
  --profiles experiments/qualification/profiles.json --profile linux \
  --action reconcile --workers 4 --samples 3 \
  --cache /tmp/vizzi-shared-source-cache/linux \
  --output /tmp/vizzi-source-linux-reconcile.json
```

Repeat for `warm`, `reparse`, and `pytorch`. These tools read original archives and
never build or edit them. A context revision forces real parsing into an empty phase
cache; source bytes remain unchanged. Check captured counts/bytes, publication outcomes,
old pins, identity agreement within each implementation, and actual parse/cache counts.

New `shared_sources` counts allocation reuse after fresh reads/hashes. It must not be
reported as skipped reads or unread retention. Keep digests from different analyzer
implementations separate; the source-storage change preserves identity composition but
conservatively changes analyzer fingerprints.

Sampled RSS, process high water, engine time, and core query timing have different
endpoints. This probe omits watcher/HTTP/browser work. Recheck real browser interaction
with a prepared cache and the usual software-renderer procedure to observe recovery
outliers; five edit samples do not establish a latency tail or baseline-laptop result.
