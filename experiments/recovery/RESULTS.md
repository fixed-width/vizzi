# Foreground edits during periodic recovery

Periodic verification now runs as one private candidate while ordinary hints continue
through foreground publication. Across ten writes per repository, each following an
observation of a distinct active recovery pass, **Linux p50/max is 119.9/139.3 ms and
PyTorch is 95.8/106.1 ms**. All twenty engine observations reopen and parse one file,
without promoting those attempts to full reconciliation. Both final snapshots agree
with independent full capture.

The [scheduling reference](../../docs/prototype/RECOVERY.md) defines the bounded journal,
fresh overlay, latest-ticket checks, worker reservation, serial fallback, and status
fields. Full discovery for ambiguous/directory/rescan/overflow requests remains mandatory.
This changes scheduling; it does not eliminate full source verification or large-file work.

## Method and observations

The [procedure](README.md) uses the same pinned archives, source manifests, language
assignments, exclusions, prepared analysis caches, four-worker budget, and opt-in
512 MiB syntax retention as the preceding [windowed run](../windows/RESULTS.md).
Linux includes 66,066 analyzed files and 1,495,484,792 source bytes; PyTorch includes
10,578 files and 186,092,681 bytes, with 10,147 analyzed and 431 CUDA source-only files.
Original source-manifest hashes match that preceding run. No analyzed project is built
or executed; edits affect disposable copies and unique UUID comments prevent old probe
facts from becoming cache hits.

The reference host is an i7-13700H with 20 exposed logical CPUs and about 62.5 GiB RAM,
Rust 1.99.1, Node 26.13.0, Playwright 1.64.1, and software-rendered headless Chromium 156
at 1440×900. Timing runs are sequential with warm OS caches and no competing builds,
tests, mutation jobs, or benchmarks. Cache preparation is excluded.

Each profile retains three four-second steady runs, three batch-refresh motion runs,
ten ordinary writes, picking, and one large-header opening/edit. Ordinary writes start
immediately after Node sees `recovery.running: true` on a new completed-pass count.
A scan may finish between that response and the actual write: this is observed overlap,
not instrumentation proving every write/scan ordering. Controlled held-parser tests
establish actual concurrency separately. Browser clients continue using SSE; Node's
status polling is diagnostic instrumentation.

| Observation | Linux | PyTorch |
| --- | --- | --- |
| Ten ordinary edits, p50 / maximum | 119.9 / 139.3 ms | 95.8 / 106.1 ms |
| Foreground engine attempts, range | 22.3–35.0 ms | 5.6–13.4 ms |
| Foreground reopened / parsed files, every sample | 1 / 1 | 1 / 1 |
| Accepted recovery counts seen before writes | 9 through 18 | 12 through 21 |
| Previously accepted full verification phase, range | 1,103.6–1,305.3 ms | 192.9–246.5 ms |
| Previously accepted fresh overlay phase, range | 19.9–27.1 ms | 1.6–3.8 ms |
| Overview rAF p99, every steady/refresh run | 16.8 ms | 16.8 ms |
| Overview rAF intervals over 50 ms | 0 | 0 |

Write-to-visible latency ends after matching source DOM and two subsequent rAF callbacks;
engine times exclude debounce, transport, and painting. Percentiles use nearest rank.
Ten samples do not establish tails: the raw p95/p99 values equal each set's maximum.
Recovery phases above are the **last accepted** phases sampled before each next pass,
not the duration of the newly active pass. Nine later samples per profile retain a
one-file overlay; the first has no overlay. Their separate counters/times are not summed
with overlapping foreground work. Completed counts advancing between all writes show
that periodic verification continued to publish.

The earlier five-edit run recorded Linux 123.2/173.9 ms and PyTorch 104.7/301.3 ms,
with full reconciliation in the PyTorch outlier. The new runs meet the provisional
ordinary-edit probe target on this host. These are historical observations with a
changed scheduler and overlap selection, not a randomized paired speedup experiment
or a guarantee that future ordinary edits always finish within 200 ms.

## Large files, painting, and memory

| Observation | Linux 24 MB AMD header | PyTorch 8.4 MB Perfetto header |
| --- | --- | --- |
| Opening | 88.2 ms | 94.9 ms |
| Edit to matching DOM plus two rAF callbacks | 219.0 ms | 1,341.3 ms |
| Targeted engine attempt | 121.7 ms | 1,242.5 ms |
| Retained syntax trees reused | 1 | 0 |
| Rendered source rows / outline controls | 49 / 0 | 47 / 21 |

Both are one-file targeted attempts. The Linux operation reuses a syntax tree, unlike
the preceding 892 ms windowed observation. Perfetto does not reuse a tree and is slower
than the preceding 744 ms observation. The report does not identify why its retained
state was unavailable, nor isolate admission, concurrent work, and parse variation.
Do not attribute these differing tree states or timings solely to scheduling. Large-file
admission, working sets, and necessary parsing remain open performance work.

Neither header operation records a long animation frame; its maximum rAF interval is
at most 16.8 ms. Overview draw CPU p99 is at most 0.5 ms in all twelve runs. These are
software reference-renderer and scheduling observations, not GPU/physical presentation
timings. The largest recorded document-window bodies remain 17,920 and 58,113 bytes,
with summary projections and the existing bounds intact.

Native lifetime high water is 2.615 GiB Linux and 0.935 GiB PyTorch; point renderer heap
samples are 10.0 and 10.9 MiB. These values include preceding batch work and are not a
controlled memory comparison or peak browser-process RSS. Parser scratch/input buffers
remain outside the syntax retention quota. No whole-process cap or 16 GB laptop
qualification is established. Browser error lists are empty.

[Linux raw report](results/browser-linux.json), [PyTorch raw report](results/browser-pytorch.json),
[summary](results/summary.json), [Linux screenshot](results/browser-linux-linux-workspace.png),
and [PyTorch screenshot](results/browser-pytorch-pytorch-workspace.png) retain the observations.
The [source manifest](results/source-sha256.json) and identical binary hashes in both
reports identify the working tree measured before commit.

## Correctness and mutation evidence

The initial foreground-progress test first failed because the host integration interface
did not exist. Its held recovery parser now permits a foreground publication before
release. Further tests hold multiple foreground jobs to check the shared parser limit,
interrupt a stale overlay, recover a later silent edit by fresh reading rather than
copying foreground documents, and compare final snapshots with full capture. Coverage
also includes old source pins, existing pending hints, journal overflow, hard requests,
finished-candidate cancellation, no-op lifecycle, independent errors, retry, shutdown,
and one-worker serial recovery. A shutdown assertion first failed on stale running
metadata, then passed after joining also cleared that state.

Real directory observation tests verify registration after full foreground discovery
with a 60-second recovery interval, so a later edit cannot succeed merely by waiting
for polling. Existing continuous-write coverage now checks the separate recovery
completion counter rather than assuming the last operation was a full scan.
Formatting, **192 Rust tests**, doctests, Clippy, and **56 Node/browser checks** pass.

| Complete affected native scopes | Tested | Caught | Unviable | Timeouts | Survived |
| --- | --- | --- | --- | --- | --- |
| Application, engine, host, invalidation | 196 | 151 | 39 | 6 | 0 |

Passing unmodified baselines precede interpretation. The aggregate takes the complete
application/engine/invalidation outcomes from one full four-file run and replaces its
host outcomes with the final full host run after shutdown/watch-test changes. It does
not count duplicate host mutations. The existing directory-prefix equivalence exclusion
is unchanged; no exclusions were added.

Development survivors exposed missing pending-journal, old-token, overlay interruption,
finished-candidate, no-op lifecycle, and watch-registration tests. Redundant status/waiting
logic was simplified. Timeouts break bootstrap/publication/cancellation progress and are
separate from assertion catches. Unviable substitutions are build failures. An initial
invocation without all features failed its baseline; none of its mutations are counted.
[Mutation details](results/native-mutation.json) preserve commands, per-file counts,
non-caught outcomes, source hashes, and development notes.

Grammar/dependency internals, serialization macro bodies, OS event delivery, browser
execution, and benchmark logic are outside these native mutation scopes. Zero survivors
is evidence rather than proof of correctness. No dependencies or unsafe-code exceptions
were introduced.

Finite latest-ticket windows are still required under continuous mutation. Hard discovery,
blocking I/O, final snapshot merging, lazy identity indexing, overview transfer, and
large-file analysis retain their existing scaling and cancellation limits. The next
resource investigation should address active large-file working sets and admission
before expanding compiler contexts or choosing production memory/renderer defaults.
