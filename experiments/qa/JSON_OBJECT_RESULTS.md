# Generic JSON-object comparison

For contributors deciding whether simpler JSON enforcement helps repository QA.
**Generic objects restore JSON syntax but worsen content and do not satisfy the host
contract.** Raw content scores 8/24 versus 14/24 for the full schema. Keep the full-schema
default. This closes the simple constraint-relaxation check without editing weights.

## Fixed inputs and paired outcome

The [contract](../../docs/qa/JSON_OBJECT.md) and [cohort](library/json-object/cohort.json)
precede generation. The same twelve known development cases run twice in each condition:
48 responses, alternating first-arm order by case and reversing it in pass two. Delivery
remains excluded for its earlier wording ambiguity; all eight real-source reservations
remain unrun. These are source-aware author grades on synthetic cases, not held-out accuracy.

Both requests retain response_format.type=json_object; only its schema property is
omitted in the generic condition. No message, source, model, sampling or instruction
changes. All twelve request pairs render byte-identical prompts through /apply-template
before generation; all 24 generated pairs have equal prompt-token counts. The pinned
runtime interprets this schema-free response type as a generic object, not the required
answer/status/citations structure.

| Measurement | Full schema | Generic object |
| --- | ---: | ---: |
| Responses / parsed JSON objects | 24 / 24 | 24 / 24 |
| Raw content correct | 14/24 | 8/24 |
| Positive-case code content correct | 14/18 | 4/18 |
| Explicit missing-source abstention | 0/6 | 4/6 |
| Existing host schema valid | 24/24 | 0/24 |
| Requested key order | 24/24 | 0/24 |
| Native usable reference binding | 24/24 | 0/24 |
| Complete structured tasks | 10/24 | 0/24 |
| Median response time | 3.73 s | 5.85 s |
| Empirical p95 response time | 5.98 s | 9.24 s |
| Median first output | 0.80 s | 0.78 s |
| Median output tokens | 60.5 | 115.5 |

All 24 full-schema controls exactly reproduce the earlier decoding run, including request,
rendered prompt, source, messages, raw output and native result; their grades are reused.
All twelve generic repeat outputs are byte-identical after identity checks; their grades
are reused from twelve manually reviewed unique outputs.

Two unique missing-source cases improve: versions and pins now explain that the decisive
definition is absent. Five unique positive cases worsen: all three complete version
variants, plus changed and permuted pin release. Each change repeats twice: four paired
content gains and ten losses, not fourteen independent observations.

## What generic objects contain

Routing objects contain an answer plus source_labels arrays of quoted numbers, without
the required status/citations fields. Original and changed routing facts are correct;
permuted and missing routing still invent a validate/pending receiver path, as in the
schema condition. Human-readable source labels are not converted into native citations.

Version and pin objects often use status_insufficient_evidence as the status value,
along with explanation, answer or source_labels fields. The required status enum is
answered/insufficient_evidence, and those extra/missing fields violate the host contract.
No values or keys are repaired. The native validator rejects these objects as returned.

The failures extend beyond shape. Version-original quotes the visible greater-than
operator but denies that an error relation is defined, answers None, and adds an unsupported
positive-integer assumption. Version-changed also quotes its operator without identifying
it as the error trigger. Version-permuted denies that the complete supplied guard body is
present. Merely quoting an operator while refusing the requested relation is not credited
as the required code fact.

Pin answers similarly demand allegedly absent validation definitions or execution
context, even though the relevant code is supplied and the question already stipulates
an error result. They do not give the requested unconditional/conditional call conclusion.
The missing-pin case correctly explains that release/validate alone cannot establish the
behaviour of absent finish. Full [grades](results/json-object/grades.json) retain those
judgments, including the prior bare-No rule and separate status/citation issues.

## Interpretation

The [earlier omitted-format condition](DECODING_RESULTS.md) produced prose rather than
JSON. Generic-object enforcement fixes that syntax failure, but does not improve the
overall code/evidence behaviour or yield a usable typed response here. The full schema
remains the strongest of these tested contracts under the unchanged prompt.

This does not establish an optimal JSON prompt: the relaxed condition receives no new
schema examples or explicit field instructions. A different prompt would be a separate
intervention. Neither the syntax gain nor the content loss locates a weight mechanism.
Remaining code/evidence misreads are the next model-side investigation target; this run
does not justify weakening validation, editing tensors or adding a product QA interface.

Greedy cached repeats are not independent samples or latency qualification. The library
retains its two/three-excerpt count cue and needs count-matched controls before a later
qualification. Real-code reservations have no model outputs. First-output timing includes
JSON preamble and is not necessarily the first user-visible answer word.

## Verification and retained evidence

All 48 requests finish normally with zero HTTP/runtime errors, parse errors or token-limit
truncations. One whole-cohort peak is 1.118 GiB process-tree PSS at 250-ms sampling, with
no reported sampling errors. Observed wall time is 232.0 seconds; startup 0.94 seconds.
No per-arm peak, enforced cap, physical laptop or editor workload is claimed. The model
and both indexes close afterward.

The base Q4 hash remains
`94d3009ee9cfca48d7d852bf0fe6c14c6f4f2506e818ca63f8c873fa46320a0a`. Conditions stay at
b12110, thinking off, two CPU/prefill threads, nice 10, context 8,192, batches 512/128,
temperature 0, seed 42, 300-word instruction, 1,024 output tokens and 360-second timeout.

Before inference, 453 workspace Rust tests/doctests, formatting, all-target Clippy and
sixteen serial Node test files pass. The new generic test fails before implementation;
the HTTP stub verifies the exact schema-property omission and earlier defaults.

Focused mutation testing covers the complete request builder and paired-study module:
**260 caught, fifteen survivors, zero timeouts/errors/no-coverage**. The
[subsequent exclusion correction](MUTATION_ERRATUM.md) identifies fourteen equivalents and
one behavioural coverage gap; original records remain unchanged. Raw
score is 94.55, with exit 1 at threshold 100. [Each equivalent](results/json-object/mutation-equivalents.json)
has its source hash, ID, location, replacement and narrow reason; survivors are not
counted as caught. Streaming/server/runner orchestration has separate integration evidence
outside mutation scope. Mutation jobs use one low-priority worker. Rust/dependencies are
unchanged, so no new native mutation cohort runs.

[Artifacts](results/json-object/) retain exact request/rendered pairs, raw responses,
[assessment](results/json-object/assessment.json), [reproduction](results/json-object/reproduction.json),
grades, source preparation, [integrity](results/json-object/integrity.json), resources and
validation/mutation evidence. [Checksums](results/json-object/checksums.json) preserve
payload identity; all earlier archive checksums remain unchanged. The
[procedure](JSON_OBJECT.md) describes reproduction.
