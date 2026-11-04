# Review analysis integration validation

Admitted review capture now applies the existing raw syntax adapters through the
shared native analyser executor and worker gate. The
[reference](../../docs/prototype/REVIEW_ANALYSIS.md) defines plan identity, source
ownership, cache/admission behaviour and deferred coverage. This slice adds no
comparison wire routes or linked review UI.

## Behavioural evidence

The first mixed-language test failed against the unimplemented `analyze` operation.
Tests through the candidate/lease interface now check:

- Rust/Python declarations on both held sides, shared content/context cache hits and
  unchanged admitted source charges;
- explicit C/C++ assignment, source-only ambiguous headers, readable documentation
  and analysis exclusions without invented facts;
- working refresh retaining exact baseline facts, required analysis before publishing
  an analysed refresh, and rejection of a different plan;
- shared input-admission refusal producing deferred coverage with readable source,
  followed by a new comparison retry after capacity is released;
- analyser errors/panics, cancellation and supersession during parsing preserving the
  active comparison and releasing private source/worker ownership;
- the host entry using its exact engine, plus concurrent foreground and review parsing
  within one configured aggregate allocation.

Worker-gate tests additionally exercise one background allocation shared with recovery,
foreground use of remaining capacity, single-worker serialisation, cancelled waits
and zero-job attempts without a background reservation. Existing engine regressions
verify cold/warm cache reuse, one-file invalidation and syntax/source correctness after
the executor extraction.

A failing regression also exposed simultaneous foreground parser panics escaping
through an unjoined scoped worker. The shared runner now joins every worker before
propagating errors; the regression verifies an engine error and released allocation.
All 380 all-features workspace tests, doctest checks, formatting and Clippy pass.

## Frozen review inputs

The diagnostic's `--analyze` mode uses an external disposable cache and one worker.
Both frozen variants retain the same two changed paths and three/two Rust hunks while
exposing `compile_file` and `read_file` declarations on the selected Rust source.
The added document has no syntax contexts/declarations and retains its absent base.
Exactly one pair remains admitted after publication, with no outstanding byte
reservation or active worker. Fixture contents/staging still verify afterwards.

The [recorded fixture results](results/analysis-fixtures.json) include contexts,
declarations, ownership, worker counters and work measurements. This is functional
evidence; debug-build timings are not latency distributions or performance qualification.
Reproduce via [the fixture guide](README.md#check-native-git-capture) with both
`--require-admission` and `--require-analysis`.

## Verification scope

The [mutation report](results/analysis-mutation.json) records module scopes, commands,
source/test hashes and raw outcome categories. Runs use one mutant, one compiler/
jobserver task, serial tests and lower priority. Timeouts and unviable cases do not
count as mutations caught by assertions. Invalid allocation/publication mutations can
cause non-progress in initial host indexing; these are separate timeout evidence.

The shared runner's live tree-retention counters require the existing syntax-reuse
tests as well as the focused review suite; follow-up scope includes those behaviours.
The broad pass recorded 90 catches, 34 unviable cases, 11 timeouts and eight survivors
across 143 cases. All eight survivors were caught by the expanded integration pass:
37 caught, ten unviable, two timeouts and zero survivors across 49 cases. After the
multi-worker panic fix, the final full shared-runner pass recorded 18 caught, one
unviable, one non-progress timeout and zero survivors across 20 cases. These cohorts
overlap and their counts must not be added. No exclusions were configured.
Generated mutations remain incomplete evidence. Configured/compiler-backed language
semantics, comparison transport, browser response ordering and usability are outside
this slice's acceptance.

Next are bounded application/transport queries that preserve token, side and document
version, followed by linked review views and the complete good/bad task walkthrough.
