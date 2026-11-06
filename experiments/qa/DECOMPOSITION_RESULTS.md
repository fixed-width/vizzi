# Focused questions and output order: diagnostic results

For contributors deciding whether guided questions or response order explain the fixed
2B model's failures. Answer-first helps two narrow tasks and reduces false refusals, but
does not repair broad workflows or basic control-flow/method-ownership errors. This is
insufficient evidence for product integration or repository-specific training.

## Frozen comparison

The [question library](library/decomposition/cases.json) contains the three exact maintainer
controls and twelve source-aware agent subquestions. Each question uses its parent's
identical whole curated source, not a smaller targeted excerpt. The source revision,
packs, Qwen3.5-2B Q4 weights, llama.cpp b12110 runtime and native selection engine are
unchanged from the [complete-source diagnostic](SOURCE_PACK_RESULTS.md).

Every question is asked twice. Only the JSON schema's property/required order changes:
`status, answer, citations` versus `answer, status, citations`. Prompt messages, source,
field meanings/types, eight ID citations, 300 words/1,024 output tokens, 2,000 answer
characters, temperature 0, seed 42 and no-thinking settings stay fixed. The native host
validates source identities and citations without depending on JSON property order.

Preparation audits facts/linkage and freezes source/code/data/binary hashes before new
outputs. Pair order alternates. Requests run serially at `nice -n 10`, with two CPU/prefill
threads, no GPU/offload, 8,192 context tokens and a 360-second timeout. No inspected source
is executed. Correctness checks finish before inference; no Rust logic or dependencies
change, so no redundant native mutation run is added.

The backend honors the requested field order in **30/30 responses**. All paired prompts
and evidence hashes match, and every broad/narrow question within a family has the same
source hash. The three status-first broad controls reproduce the previous complete-source
controls' prompts, evidence and **raw output byte-for-byte**; see
[control reproduction](results/decomposition/control-reproduction.json).

The subquestions name classes, operations or failure conditions. They are expert guidance
on known source, not unfamiliar-user questions, an automatic decomposition planner or a
held-out accuracy benchmark. No subanswers are supplied to another request or synthesised
into a final workflow. Fact counts overlap across questions and are not independent samples.

## Results: 30 paired responses

Grading is preliminary manual work by the source-aware question/pack author. A complete
task needs correct material facts/status, no unsupported claims and supporting valid
citations. Correct statements are recorded separately from contradictory caveats and
wrong citations. Equivalent wording is accepted; ambiguity and interpretation choices
are preserved in [the review](results/decomposition/review.md).

| Measurement | Status-first | Answer-first |
| --- | ---: | ---: |
| Complete original workflows | 0/3 | 0/3 |
| Complete focused tasks | 2/12 | 4/12 |
| Supported broad facts | 4/13 | 5/13 |
| Supported focused facts | 10/18 | 13/18 |
| False refusals, broad | 1/3 | 0/3 |
| False refusals, focused | 2/12 | 0/12 |
| Answers with unsupported claims, all questions | 12/15 | 9/15 |
| Native reference checks | 14/15 | 15/15 |
| Generated order compliance | 15/15 | 15/15 |

Of twelve focused pairs, answer-first helps **two**, hurts **zero**, both succeed on two,
and both fail on eight. Both orders succeed on the native handler-interface question and
the Start-request/LiveSnapshot-reply question. Answer-first additionally completes the
normal Task creation-method question and the unavailable-history rejection question.
The original three workflows remain incomplete in both orders.

Source identity is not citation support. For example, answer-first correctly describes
the `efuns!` registration/dispatch mechanism but cites the example row and Markdown
instead of the macro-definition block needed to support the dispatch relationship. Its
factual improvement is credited, while complete task success still fails.

The four incorrect broad creation/commit answers still contain invented caller or message
ownership. Answer-first also turns the broad efun refusal into an asserted procedure that
wrongly says defining the prototype is sufficient. Removing a false refusal is not enough
to establish a correct answer.

## Remaining failure patterns

- **Equivalent conditions treated as different:** status-first describes `FutureVersion`
  for `current_version < changeset_version`, then claims the question about
  `changeset_version > current_version` is not answered. It also invents source label 6.
- **Correct fact followed by refusal:** the unavailable-history status-first answer names
  `HistoryUnavailable` and then claims the outcome is unknown. Answer-first states the
  rejection directly and cites the correct validation block.
- **Method ownership:** both receiver answers invent `Committer::dispatch`; the provided
  lexical scope and source identify `Committer::process` calling `Committer::commit`.
- **Early-return reasoning:** both read-only answers claim a new version is created even
  though `publish` returns before `Version::new()` and snapshot application.
- **Operation conflation:** both merge answers describe write-history/read-invalidation
  scanning as merge validation, rather than folding against committed values.
- **Retry control flow:** both physical-effect answers describe breaking the attempt loop
  after rejection, although conflicts continue through admission/backoff to another attempt.
- **Eviction:** status-first mistakes the unavailable-history rejection condition for an
  eviction trigger. Answer-first gives the live-snapshot-empty condition but still invents
  the receive-method name and omits its relevant citation.

Some correct core answers use loose terminology. The handler-interface score does not
require the unasked Rust `Result<()>` spelling. The typed `LiveSnapshot` reply is accepted
when "transaction handle/state" is plausible release/world-snapshot shorthand, while an
explicit new `Transaction` inside that snapshot is rejected. These judgments are recorded
for independent review rather than silently changing frozen questions.

## Resources and constraints

All 30 outputs parse, with no runtime errors, timeouts, context overflows or truncations.
Maximum output is **215 tokens** and **921 answer characters**, below the 1,024-token and
2,000-character caps. The remaining failures are not explained by those hard limits.

Observed process-tree peak PSS is **2.22 GiB**, sampled every 250 ms with no observer
errors. This is one repository index, runner and CPU inference server on the same
i7-13700H / 62.5 GiB host, not an enforced cap or a 16 GB device qualification. The
temporary model server closes after the run; no Vizzi viewer/editor is opened.

Median full response is **16.4 s / 16.0 s**. Prefix caching reuses almost all source on
many paired/family requests. Broad controls begin each family and pay initial prefill;
these timings cannot establish that narrower questions or field order are faster.
Cached-token counts and per-response times are preserved. The first output measurement
includes JSON framing, not necessarily a useful answer word.

## Decision and verification

Keep answer-first as a promising diagnostic option, not an approved product answer path.
It improves directness and some supported narrow answers in this small fixed cohort,
while incorrect claims and source-link errors remain common. Guided decomposition helps
some local lookups but does not make the original workflow reliable, and the tool has
not generated or assembled those subquestions automatically.

The next useful comparison is a bounded alternative CPU-capable model on these same
frozen error patterns and source packs, under a declared memory/thread budget. Avoid
further prompt-only polishing or an automatic planner until basic method ownership and
control-flow answers are reliable. Any later acceptance evaluation needs fresh maintainer
questions and independent review; these cases are development diagnostics now.

Formatting, **453 workspace tests including doctests**, all-target workspace Clippy and
**nine serial Node test files** pass. The new ordering, study-validation and paired-scoring
tests fail before implementation and pass afterward. Streaming tests verify real request
schemas and parsed key order. No unsafe code/dependencies or Rust source changes are
introduced. Existing native selection/provenance mutation evidence remains **52 caught /
3 unviable**, with no survivors/timeouts in that prior relevant scope; untouched ranking
and JavaScript mutations are not qualified by cargo-mutants.

[Reproduction procedure](DECOMPOSITION.md). Preserved artifacts include
[assessment](results/decomposition/assessment.json), [raw prompts/responses](results/decomposition/answers.jsonl.gz),
[grading review](results/decomposition/review.md), [source/code freeze](results/decomposition/prepared.json.gz),
[constraint observations](results/decomposition/constraints.json), runtime logs, memory,
validation and checksums. Gold facts and author notes do not cross the inference boundary.
