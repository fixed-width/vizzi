# Response-format ablation

For contributors deciding whether the evidence-use failures justify a weight edit.
**Removing the schema changes which errors occur but gives no aggregate content gain.**
Both conditions score 14/24 on raw content. Two unique cases improve and two worsen,
each repeated on pass two. Every unconstrained response is prose rather than valid JSON,
so none is usable through the current structured/native-bound interface. Keep the default
schema. No model weight, dependency or reserved evaluation output changes.

## Controlled inputs

The [contract](../../docs/qa/DECODING.md) precedes answers. Twelve existing development
cases cover routing, version predicates and pins: original, changed, permuted and missing
definitions. The delivery wording ambiguity is excluded before generation. Eight real-code
reservations remain unrun. These are known synthetic cases and source-aware author grades,
not independent accuracy or real-code acceptance.

There are two passes and two conditions, 48 serial requests. First-arm order alternates
per case and reverses on pass two. Only `response_format` is omitted; literal messages
still request JSON without new examples or schema/order instructions. Shape, field order
and schema answer-length enforcement are removed together. This is not a comparison with
an independently optimised unconstrained prompt.

Before generation, all 24 exact request bodies are posted to `/apply-template`. All
twelve pairs render byte-identical prompts. All 24 generated pairs also have equal
prompt-token counts. The [pinned server reference](https://github.com/ggml-org/llama.cpp/blob/cd959bae99ff4bed22bef2604f45241c09bbb8c8/tools/server/README.md)
documents the response-format/template facilities; archived bodies and prompts retain
the actual observation, beyond just matching client messages.

Both use unchanged base Q4 SHA-256
`94d3009ee9cfca48d7d852bf0fe6c14c6f4f2506e818ca63f8c873fa46320a0a`, b12110, thinking
off, two CPU/prefill threads, nice 10, context 8,192, batches 512/128, temperature 0,
seed 42, 300-word instruction, 1,024 output tokens and 360-second timeout.

## Paired results

| Measurement | Schema enforced | Response-format omitted |
| --- | ---: | ---: |
| Responses | 24 | 24 |
| Raw content correct | 14/24 | 14/24 |
| Positive-case code content correct | 14/18 | 12/18 |
| Explicit missing-source abstention | 0/6 | 2/6 |
| JSON parsed / schema valid | 24 / 24 | 0 / 0 |
| Requested key order | 24/24 | 0/24 |
| Native usable reference binding | 24/24 | 0/24 |
| Complete structured tasks | 10/24 | 0/24 |
| Median response time | 4.39 s | 8.01 s |
| Median first output | 0.90 s | 0.80 s |
| Empirical p95 response time | 7.25 s | 12.71 s |
| Median output tokens | 60.5 | 146 |

All 24 schema controls exactly reproduce the prior evidence-use outputs, source,
messages, native references and finish state, so their grades are reused. Eleven
unconstrained repeat outputs are byte-identical and reused after identity checks.
Thirteen new/changed outputs are manually reviewed. Routing-original is the only changed
unconstrained repeat; its factual judgment stays correct.

Four paired responses gain content correctness and four lose it. These are two unique
cases each, repeated twice, not eight independent examples:

| Unique case | Effect of omitting schema |
| --- | --- |
| Routing, original source permuted | Improves: correctly reads dispatch calling validate instead of inventing a pending call. |
| Pins, decisive definition missing | Improves: explains that helper definitions cannot establish missing finish behaviour. |
| Versions, changed predicate | Worsens: opens with correct less-than, then contradicts the guard and its own conclusion. |
| Pins, changed conditional release | Worsens: loses correct No and falsely denies the explicit validation-result check. |

Other errors persist: missing routing still yields an invented receiver/pending call;
missing guard source still yields unsupported comparison rules; unconditional pin release
is misread in both conditions. Lifting the schema does not remove those failures.

Correct code prose can also end with false insufficient-evidence text. Unconstrained
versions-permuted describes the greater-than rule correctly, then says the relationship
cannot be established. Under the predeclared metric, code content is credited separately
from status/calibration. A missing-source answer needs an explicit `rawAbstention`
judgment, not a blank output or bare No credited through an empty positive-fact checklist.

All prose is retained verbatim. Zero native usable bindings means there is no parsed
structured answer to validate; it does not mean every prose claim is false. No JSON
object is extracted, no fence removed and no fields repaired. Manual content correctness
is distinct from a safely bound product response.

## Interpretation and limits

Equal effective prompts with different results show an output-contract effect. They
do not locate a circuit or isolate every schema, generated-prefix, output-length or
sampling-trajectory contribution. Longer prose creates more opportunities for false
claims; full responses are graded, not only opening sentences. Dropping the constraint
is not a useful replacement here.

A generic JSON-object versus full-schema comparison could separate broad JSON effects
from particular field/order constraints. Format-independent misreads, such as unconditional
pin release, remain separate potential weight/capacity targets. Neither finding justifies
an arbitrary tensor edit or product QA integration.

Greedy prefix-cached passes are replays, not independent samples or a latency distribution.
Missing/full cases retain the two/three-excerpt count cue; future qualification needs
count-matched controls and a new pre-edit freeze. No reserved answers are inspected.
First-output time includes JSON preamble for the schema condition, not necessarily the
first user-visible answer word. Larger real-code packs are not represented by this timing.

## Verification and artifacts

All 48 requests finish normally: zero HTTP/runtime errors and token-limit truncations.
There are 24 JSON parse failures, all in the unconstrained condition. The one whole-cohort
peak is 1.130 GiB process-tree PSS at 250-ms sampling without reported sampling errors.
Observed cohort time is 314.2 seconds, startup 0.85 seconds. No per-arm memory, enforced
cap, physical laptop or editor workload is claimed. Owned server/index processes close.

Before inference, 453 workspace Rust tests/doctests, formatting, all-target Clippy and
fifteen serial Node test files pass. HTTP-stub tests verify the actual single-field
request change and unmodified non-JSON retention. Request/module, rendered-input,
task-consistency and empty-abstention regressions are observed failing before fixes.

Mutation scope is the complete new request builder and decoding-study module:
**234 caught, thirteen documented equivalents, zero timeouts/errors/no-coverage**.
[Each survivor](results/decoding/mutation-equivalents.json) is bound to source hash,
location, replacement and a narrow reason: redundant rejection/pairing guards, parity-
preserving order changes, irrelevant invalid-input fallback keys or diagnostics. Raw score
is 94.74 and the raw command exits 1 at threshold 100; survivors are not called caught.
Streaming/server/runner orchestration is outside mutation scope with separate integration
checks. No new native mutation cohort runs because Rust is unchanged. Mutation work uses
one low-priority worker.

[Artifacts](results/decoding/) retain request/rendered-prompt pairs, raw responses,
[grades](results/decoding/grades.json), [assessment](results/decoding/assessment.json),
[control reproduction](results/decoding/reproduction.json), source preparation, memory
observations and initial/expanded/final mutation reports. [Integrity](results/decoding/integrity.json),
[validation](results/decoding/validation.json) and [checksums](results/decoding/checksums.json)
record checks and limitations. Historical payload checksums remain unchanged. The
[procedure](DECODING.md) describes reproduction.
