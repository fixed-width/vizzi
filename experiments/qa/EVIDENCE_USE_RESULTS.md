# Evidence-use development diagnostic

For contributors selecting a behavioural failure to investigate before editing weights.
**The unchanged 0.8B model makes repeatable source-use errors on small inputs.** It follows
changed receiver names and comparison operators when the decisive excerpt is first, but
one unchanged call path becomes invented after excerpt order/labels change. It also gives
specific answers when the decisive definition is absent. No weight mechanism is located.

There are 32 development responses: sixteen cases in two passes. All sixteen second-pass
answers are byte-identical after source/message/reference checks. Eight real-source
questions in Vizzi/PyTorch are reserved and receive **zero model generations**.

## Source audit and grading limits

The [contract](../../docs/qa/EVIDENCE_USE.md) precedes outputs. A post-output source/question
review identifies ambiguous wording in the delivery family: it calls an in-memory vector
marker a physical effect without defining that representation to the model. The original
library, answers and intended-reading grades stay intact. A separately labelled
[clarity-checked assessment](results/evidence-use/clarity-assessment.json) excludes that
family. This is a post hoc interpretation check, not a predeclared primary benchmark.
The [oracle review](results/evidence-use/oracle-review.json) records the reason.

All scoring is preliminary manual work by the source-aware fixture/question author.
The synthetic examples are diagnostic input, not real-code acceptance. The reserved cases
are also agent-authored; their source/gold was audited, and their model outputs remain
unobserved. Existing repository exposure and unknown pretraining membership prevent a
claim of independent repository-level hold-outs.

## Repeatable observations in three clearer families

The following shows complete-task outcomes for each unique case. Each outcome repeats
exactly in pass two. Complete tasks require facts, expected status, supported citations,
native source binding and no unsupported assertions.

| Family | Original source | Answer-changing source | Original source permuted | Decisive definition withheld |
| --- | --- | --- | --- | --- |
| Routing | Pass | Pass | Wrong receiver/call and false insufficient status | Invented receiver/call |
| Version predicate | Pass | Pass | Pass, including new supporting label | Unsupported, inconsistent predicate |
| Pin release | Wrong No and false insufficient status | Correct No but false insufficient status | Correct Yes, wrong supporting label | Insufficient status with an unsupported bare No |

For routing, the original receiver handles Apply and calls its validator. The model gets
both original and renamed-role source states right. Permuting the **same range IDs/bytes**
makes it claim that the validator handles Apply and calls `pending`, which merely returns
zero. The decisive receiver definition is still present at label 3. Native validation
accepts its cited label 1 because that excerpt exists; the claim is not supported by it.

For versions, both `requested > committed` and the changed `requested < committed` are
read correctly, and the permuted original uses label 3 correctly. With the guard missing,
the model still supplies contradictory greater-than/less-than-or-equal rejection rules.
This is a counterexample to a universal inability to read conditions or cite later excerpts.

For pins, an unconditional release after validation gets No. A guarded release on success
gets a factually correct No with a false insufficient-evidence status. Permutation yields
a correct Yes for the unconditional case but cites the validator rather than `finish`.
The missing-definition answer is only No despite its insufficient status; the grader
records that as an unsupported yes/no conclusion rather than an explanation of missing
evidence. The detailed missing-routing/version errors stand independently of that judgment.

Within this post hoc three-family subset, 10/18 answerable responses complete their tasks.
There are six false insufficient-evidence responses. None of six missing-definition
responses meets the full abstention rubric, including the explicit bare-No judgment above.
These counts are replays of twelve unique cases, not 24 independent examples.

The full [32-response assessment](results/evidence-use/assessment.json) retains 12/24
answerable task successes under the intended delivery interpretation, with the ambiguity
flagged separately. It is not a clean accuracy estimate. All
[per-response grades](results/evidence-use/grades.json) and raw answers remain reviewable.

## What this establishes

The predeclared repeatability rule requires the same error class in both passes of at
least two distinct development cases. In the clearer subset, factual errors repeat in
two cases across routing/pins; status errors in five cases; unsupported claims in five;
and citation-support failures in three. These are overlapping categories, not additional
independent errors or located neural circuits. No native binding/runtime failure repeats.

The useful target is **using the supplied evidence consistently, including uncertainty
and supporting-source choice**. Changing source semantics succeeds in some conditions;
changing its presentation breaks others. These observations do not distinguish a weight
obstruction from output grammar, prompt interaction, attention/position effects or limited
capacity. A controlled decoding/output-format comparison is a useful next diagnostic
before selecting tensors or claiming that a behavioural edit adds ability.

Missing cases have two excerpts while full cases have three. Future qualification needs
count-matched controls and a new pre-edit freeze; an edit could otherwise learn that
presentation shortcut. The eight current reservations are starter real-code questions,
not a sufficient qualification suite on their own. Do not tune on their contents/results.

## Inputs, resources and verification

The [library](library/evidence-use/README.md) contains four controlled Rust families in
two explicit synthetic states and four separate real-source families in Vizzi/PyTorch.
States are content-bound fixtures, not Git commits. Every variant stays with its family;
case IDs, families and full-file source bytes cannot cross the two splits. QA capture
remains Rust/Python; C headers are outside this study's scope.

Native preparation checks all 24 contexts against pinned full-file SHA-256, captured
content/snapshot identities, exact source ranges and required anchors. Model messages
contain only development questions/repository and captured excerpts; expected facts,
range keys/support labels and reserved contexts are not model inputs. Run-time selection
is compared with preparation. The two fixture indexes and model server close afterward.

The model is the unchanged local base Q4 from the weight-edit comparison, SHA-256
`94d3009ee9cfca48d7d852bf0fe6c14c6f4f2506e818ca63f8c873fa46320a0a`, with the same
pinned b12110 runtime, non-thinking answer-first output, two CPU/prefill threads, nice 10,
context 8,192, batches 512/128, temperature 0, seed 42, 300 words/1,024 output tokens,
eight source IDs, existing 2,000-character answer schema and 360-second timeout.

All 32 responses finish normally with requested key order and valid native references;
no runtime/JSON errors or token-limit truncations occur. Some delivery explanations stop
mid-expression despite normal finish; that is retained, not repaired. Median full response
is 4.33 seconds, median first output 1.82 seconds, empirical p95 6.38 seconds. Median
evidence is only 349 bytes; these timings do not compare fairly with larger real-code packs.
Observed whole-run time is 135.7 seconds; startup 0.84 seconds; peak process-tree PSS
1.111 GiB at 250-ms sampling with no reported sampling errors. No memory cap, physical
laptop or editor workload is emulated; the inactive observer default is not the 4-GiB
target. Greedy prefix-cached replays do not establish a latency distribution.

Before inference, 453 workspace Rust tests/doctests, formatting, all-target Clippy and
fourteen serial Node test files pass. Eight safe Rust 2024 data fixtures format/compile.
The focused helper test and a cross-family regression are observed failing first.

Mutation testing covers the entire new `evidence-study.mjs`: **333 caught, ten equivalent
survivors, zero timeouts/errors/no-coverage**. The ten reviewed equivalents change six
exception strings or remove four early guards whose invalid inputs still fail later.
[The exclusions](results/evidence-use/mutation-equivalents.json) bind exact IDs, locations,
replacements and reasons to the tested source hash. They remain survivors in the raw
report, not caught mutations. Raw score is 97.08; the tool exits 1 at its 100% threshold.
Runner orchestration is integration-checked but outside mutation scope; production Rust
is unchanged, so no new native mutation cohort runs. All jobs use one worker at low priority.

[Archived artifacts](results/evidence-use/) retain both split libraries, prepared/source
audits, raw development messages/answers, grades, full/post hoc assessments, resource
observations, validation logs and initial/expanded/final mutation reports.
[Integrity](results/evidence-use/integrity.json),
[validation](results/evidence-use/validation.json) and
[checksums](results/evidence-use/checksums.json) retain the checks and limits. Historical
experiment payload checksums are unchanged. [The procedure](EVIDENCE_USE.md) describes
reproduction. No model edit, new dependency or product QA interface is introduced.
