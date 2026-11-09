# Cold semantic lookup investigation

For maintainers assessing the initial definition delay, 2026-11-08. The earlier whole-request
measurement attributed 10.48 seconds to cold setup/inference without isolating its stages.
This investigation uses the same known GsMeter LUFS and Minitosis processor calls, optimised
native host, captured-source mirror, one low-priority analysis worker and disabled builds,
proc-macros and check-on-save. Results are observations from individual sequential sessions,
not performance distributions or independent usefulness qualification.

## Bottleneck

A transparent [LSP timing peer](profile-lsp.py) records methods, IDs, configuration, resource
samples and timestamps; it does not store didOpen/didChange source text. It can patch startup
configuration for controlled diagnostic variants. The normal benchmark configuration does
not use that peer. rust-analyzer's [hierarchical profiler](https://rust-analyzer.github.io/book/contributing/)
identified standard-library declaration/import-map construction in the first definition.

The light-timing baseline measured:

| Stage | Observation |
| --- | ---: |
| LSP initialize handshake | 13 ms |
| Project loading, through quiescent notification | about 1.0 s |
| First definition RPC | 7.51 s |
| Entire browser action | 8.61 s |
| Repeated definition RPC | roughly 0.2–0.4 ms |
| Repeated native HTTP requests | 4–6 ms |

The protocol span occupies almost the whole action. Mirroring, host checks, mapping, HTTP
and browser work together account for roughly the remaining 0.1 seconds in this run; they
are not individually isolated by the external peer. A quiescent server has finished pending
background work, but its on-demand semantic caches can remain cold. See the
[server-status contract](https://rust-analyzer.github.io/book/contributing/lsp-extensions.html#server-status).

Enabling detailed profiling added enough overhead to hit the existing ten-second request
deadline. That unsuccessful diagnostic is kept separately; its incomplete RPC spent the
budget building `core`/`alloc` import maps, parsing and expanding builtin macros. It is not
a successful latency sample or evidence of a repository build.

## Alternatives checked

All variants kept source/target correctness assertions and the same single-worker limits.

| Variant | First browser lookup | Decision |
| --- | ---: | --- |
| Toolchain rust-analyzer 1.99.1, cache 64 | 8.61 s | baseline |
| Syntax-tree cache 512 | 9.32 s | no demonstrated improvement |
| Stable standalone 0.3.3105, release 2026-11-02 | 9.19 s | no demonstrated improvement |
| Full cache priming, one thread | 12.62 s | increased total cold wait |

The standalone binary was downloaded to `/tmp`, SHA-256 verified against the official stable
release asset ([2026-11-02](https://github.com/rust-lang/rust-analyzer/releases/tag/2026-11-02); `5249da68c3f36bb77ea7d401824f5e8bd8d0a47a162daf5f72208eb7c1bf1500`)
and did not replace the user's toolchain. Full priming reduced the definition RPC to about
647 ms but spent about 11.9 seconds before readiness. It moved/expanded work within the
same foreground wait. The [configuration reference](https://rust-analyzer.github.io/book/configuration.html)
defines syntax-cache capacity and cache priming; no sysroot/dependency/type analysis was
disabled to obtain faster answers. Limited samples do not establish that variants are
intrinsically slower on every workload.

## Improvement

When a function's captured written calls load, the browser now starts at most one background
lookup for a recognizable call lacking a source-local binding. This warms the relevant
function/crate analysis while its source remains readable. It does not prime every crate,
raise concurrency or open any destination. An optional configured provider is still required.

Foreground navigation waits for pending preparation, then makes a fresh definition request.
Speculative destinations are discarded, so source/configuration validation still occurs at
selection time. Unsupported/failed preparation cannot block later attempts indefinitely.
Snapshot changes reset the scheduling state, and late/closed-view guards remain in place.
The footer distinguishes preparation from requested lookup without changing source readiness.
The native provider serializes overlapping requests instead of turning preparation into a
busy/unavailable name-match fallback; the host's existing work admission remains bounded.

A matched replay allowed twelve seconds of reading after each source fragment opened in
both the previous and updated builds:

| GsMeter measure | Previous | Updated |
| --- | ---: | ---: |
| Deliberate reading interval | 12 s | 12 s |
| Wait after selecting the call | 11.14 s | 43.8 ms |
| Fragment ready to definition response | 23.23 s | 12.14 s |

Both opened `LufsMeter::process_sample` in `gs-meter/src/lufs.rs`. The Minitosis field case
also retained its correct target/caller context. This improves interactive wait by overlapping
necessary work with reading; it does not remove the initial analysis cost. An immediate click
can still wait for cold preparation. New sessions and project-configuration changes still
need analysis. Background preparation also means configured type-aware navigation can spend
CPU/memory before a call is selected. No general cold-cache speedup is claimed.

## Verification

TDD reproduced missing warmup scheduling and the overlapping-request busy failure. Unit
checks cover one-job admission, foreground waiting, snapshot changes, invalid/wrong-snapshot
responses, provider failure/retry and each completed result state. The browser peer returns
one speculative destination and a different foreground destination: no speculative pane
opens, the fresh result wins and caller geometry/reading remain intact. Existing close,
stale-source, source-isolation and recorded-editor checks remain relevant.

Formatting, Clippy, the native workspace/doctest suite, three explicitly run real-provider
integration tests and the complete browser suite passed. The new concurrent-provider test
is part of the optional real-server cohort. The warmup scheduler mutation run caught 50 of
52 variants; two are equivalent: initial prepared state is reset on the first valid snapshot,
and the completion job is the only job that can own the pending slot. No exclusions,
timeouts or errors. The mutex change is covered by the real concurrency regression, not
by mutation qualification of the full provider implementation. Results are evidence, not proof.

[Raw observations](results/cold-lookup/summary.json),
[baseline protocol](results/cold-lookup/baseline-protocol.jsonl.gz),
[hierarchical profile](results/cold-lookup/hierarchical-profile.stderr.gz),
[matched previous replay](results/cold-lookup/reading-before.json.gz),
[matched updated replay](results/cold-lookup/reading-after.json.gz),
[native tests](results/cold-lookup/native-tests.log.gz),
[real-provider tests](results/cold-lookup/real-provider-tests.log.gz),
[browser tests](results/cold-lookup/browser-tests.log.gz),
[Clippy](results/cold-lookup/clippy.log.gz),
[mutation report](results/cold-lookup/warmup-mutations.json.gz).

The replay supports `VIZZI_HOST_BINARY` for a pinned comparison executable and
`VIZZI_READING_MS` for the controlled reading interval. Profile and replay outputs must use
fresh directories; stop other verification workloads before interpreting timings.
