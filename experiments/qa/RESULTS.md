# CPU repository-QA pilot

Observed 2026-11-05 on source baseline `5ee084d`. This record is for maintainers deciding
whether to continue the local-question experiment, not an acceptance claim for a chat UI.
See the [procedure](README.md), [library](library/README.md), [rubric](RUBRIC.md) and
[raw assessment](results/pilot/assessment.json).

The external index kept the changing-source cases current and both candidates fit the
pilot's observed memory budget. The tested retrieval/prompt/protocol combination does
**not** yet provide sufficiently reliable, fast general codebase answers. Memory fit is
feasible; retrieval relevance, interpretation, citation handling and prefill cost are
separate blockers. No weights were trained and no inspected project tooling was run.

## Inputs and controls

The seed contains 16 reusable templates and 32 concrete cases: twelve general questions
instantiated on real repositories, twelve codebase-specific questions, four unsupported
operational questions and four changing-source cases. It has 27 answerable and five
unanswerable cases, split before inference into 18 development and 14 held-out cases.
Every preferred evidence needle was audited against its pinned source; oracle excerpts
were also checked to contain those anchors. This does not establish semantic completeness
or eliminate question ambiguity.

| Source | Pin | Selected text files | Bytes | Chunks |
| --- | --- | ---: | ---: | ---: |
| Vizzi | `5ee084d9e7d6876a61af627403ca99fe208d043f` | 410 | 3,110,772 | 4,839 |
| lpc-rs | `1a4f5b65dd2a269298748df2dfd68092a169a8d9` | 931 | 10,754,851 | 22,808 |
| Changing fixture | Explicit content-state labels, not Git hashes | 3 initially | 139 initially | 4 initially |

The native scope covers Rust/Python with raw syntax boundaries and plain JS/TS/HTML/CSS,
Markdown, TOML, YAML and license text. Generated result/facilitator directories, the QA
library and LPC source are excluded. Omission counts and archive hashes are in the
[manifest](results/pilot/manifest.json). Frozen source archives contain no new gold cases.

Candidates are Qwen 3.5 0.8B/2B Q4_K_M with exact publisher revisions, byte sizes and
SHA-256 values in [models.json](models.json). The official catalogue was checked for
newer small releases; later Qwen generations found were substantially larger. Active
MoE parameter counts were not treated as resident-weight memory bounds. Runtime is the
CPU Ubuntu x64 build b12110 linked by stable llama.cpp v0.9.0, commit
`cd959bae99ff4bed22bef2604f45241c09bbb8c8`, with the release archive verified.

Both use two inference/prefill threads, lower process priority, no GPU/device/operation
offload, no image projector, offline runtime, an 8,192-token context, six excerpts under
8,000 source bytes, and a 384-token generation cap. Thinking is disabled and remained
empty in all responses. Decoding is deterministic greedy, seed 42. This differs from
the authors' recommended stochastic sampling ([0.8B model card](https://huggingface.co/Qwen/Qwen3.5-0.8B),
[2B model card](https://huggingface.co/Qwen/Qwen3.5-2B)) and is not a best-capability assessment.
The schema, prompt, request flags, source/model/native-binary hashes and raw output are
preserved. No candidate was tuned after its outputs were inspected.

There were 96 responses: 32 retrieved-context questions plus 16 preferred-source
(`oracle_source`) diagnostics for each candidate. There were no runtime or JSON parse
errors and no length-limit finishes. JSON format validity alone did not establish useful
answers. Model-provided paths, references and quotations were not repaired.

## Resource and latency observations

Linux x86_64, Intel Core i7-13700H, about 62.5 GiB physical host RAM. Tests and mutation
jobs finished before performance collection. Other desktop work was not isolated or
emulated. Models ran sequentially; each temporary server and all native index processes
closed after the cohort. Existing Vizzi viewers stayed stopped.

| Retrieved-context workload, 32 questions each | 0.8B | 2B |
| --- | ---: | ---: |
| Peak concurrent process-tree PSS while model live | 1.14 GiB | 2.21 GiB |
| Peak concurrent process-tree RSS while model live | 1.19 GiB | 2.26 GiB |
| Median first non-reasoning JSON output | 14.4 s | 34.5 s |
| Median complete inference response | 20.1 s | 46.2 s |
| p95 complete inference response | 29.2 s | 57.7 s |
| Median native retrieval | 0.71 ms | 0.76 ms |

Medians use the upper middle observation; p95 is nearest rank. First-output timing
includes the JSON preamble, not necessarily the first useful answer word. Complete
latency covers inference HTTP delivery; retrieval is measured separately. Indexes,
runner and observer contribute to memory, not just the model. Values are simultaneous
sampled sums, not sums of separate process peaks.

The memory target was an observed combined peak below 4 GiB. Both fit it. The first-output
five-second goal was missed, and complete responses were at/beyond the provisional
20-second median goal. With shorter preferred-source diagnostic contexts, medians were
9.8 s complete / 5.3 s first output for 0.8B and 22.8 s / 11.9 s for 2B. This suggests
context/prefill cost is important, but content and length differ between conditions; it
is not a controlled context-length ablation. Warm file-cache model startup was about
0.95 s and 1.46 s after streamed weight hashing; these are not cold-start observations.

The safe observer collected 8,987 samples at nominal 250 ms, with no sampling errors.
Whole-cohort duration was about 41.6 minutes. `cap_verified=false`: this was observation,
not an enforced 4 GiB/16 GB qualification. PSS excludes unmapped filesystem cache and
kernel charges; sampling can miss short peaks. Linux/PyTorch scale and physical laptop
workloads remain unqualified. See the compressed [memory record](results/pilot/memory.json.gz).

## Preliminary semantic review

All 64 retrieved-context answers received source-aware rubric review by the agent that
also authored the seed cases. This shared perspective is a material limitation. The
[per-case grades and rationales](results/pilot/grades.json) and
[readable review](results/pilot/review.md) permit independent reassessment. Diagnostic
answers were inspected qualitatively rather than assigned the primary task score.

| Primary retrieved-context review | 0.8B | 2B |
| --- | ---: | ---: |
| Required material facts supported | 24 / 48 | 28 / 48 |
| Fully successful answerable tasks, including protocol/source support | 2 / 27 | 5 / 27 |
| Fully successful unanswerable tasks/refusals | 3 / 5 | 5 / 5 |
| Partial / incorrect / abstained task results | 8 / 6 / 13 | 7 / 5 / 10 |
| Strict source checks passed, including refusals | 18 / 32 | 18 / 32 |

Supported facts, source binding and task success are deliberately separate. A response
can contain correct facts but omit required citations, or use valid source with a false
interpretation. A status of `insufficient_evidence` sometimes accompanied an actual
negative answer; status alone is not an accuracy measure. Refusals on answerable questions
numbered 14 and 11 by the raw status field. Neither candidate confidently marked an
unanswerable case answered, but some refusals invented source labels or gave unhelpful
explanations.

Preferred literal-anchor coverage was complete on only 12/27 answerable retrieved cases,
identically for both candidates. It is not semantic recall: valid alternative evidence
can answer a question without containing the preferred needle. Examples:

- The 2B model answered the validation-policy question from the preferred contributor
  guide, but interpreted “baseline” as Git comparison acquisition under ordinary retrieval.
- Ordinary retrieval missed lpc-rs's overall README purpose; preferred-source context let
  both models describe the compiler/VM/gamedriver and multiplayer worlds.
- The 2B answer invented a production retry counter by confusing a `RecoveringReader`
  test double with the compiler loop. Another answer conflated a masked diagnostic path
  with host-input access confinement. These are interpretation failures, not just missing
  literal source.
- The 0.8B model frequently refused even when a preferred excerpt contained a direct
  policy statement. The 2B model sometimes quoted the answer but still classified it as
  insufficient evidence.

Citation errors included absent citations, wrong/unavailable labels, edited text and
Markdown line wrapping. A labelled post-hoc whitespace diagnostic raises binding counts
to 19/32 and 25/32; it does not replace strict results or establish entailment. The
[diagnostic record](results/pilot/citation-diagnostics.json) preserves that distinction.

The changing-source cases produced the correct retry values 3 then 7, the renamed
`load_packet` handler and refusal after deletion for both candidates. Most still had
citation/protocol problems. Native config-refresh/delete observations were about
0.10–0.20 ms; parse/rechunk of the tiny renamed fixture was about 15–22 ms. Unchanged
chunks were reused; postings were rebuilt. This is a tiny deterministic fixture, not a
real-repository commit-speed claim.

The seed needs maintainer review. Language questions contain some language hints;
purpose/rubric granularity is imperfect; “inferred” versus “automatically verified” and
“inspect bodies” versus “match body text” can be ambiguous. Preferred excerpts may omit
caller/component linkage even when each anchor is present. Frozen cases were not edited
or removed to improve the measured scores.

## Software verification and mutation limits

Formatting, 433 workspace tests including doctests, Clippy over all targets/features,
question-library validation and model-adapter tests pass. A delayed local HTTP stub
checks that first-output timing does not consume the entire stream first. Regression
checks cover literal quotes/labels, held-context refusal, same-metadata edits, deletion,
UTF-8/line boundaries, oracle completeness, source scope and protocol/resource limits.
No unsafe Rust or new third-party Rust dependency was added. No viewer UI change was
made, so its browser suite was not rerun for this isolated experiment.

Serial lower-priority cargo-mutants runs used one mutant job/compiler task and serial
tests. Broad rounds and focused follow-ups are recorded in
[mutation observations](results/mutations.json); overlapping counts must not be summed.
The initial full run had 224 generated mutations: 103 caught, 112 surviving, seven
unviable and two timeouts. A strengthened full round had 227: 146 caught, 70 surviving,
nine unviable and two timeouts. Subsequent focused iterations strengthened bounds,
identifier splitting, selected-source identity, source labels, context expiry and
oracle-source handling. Some tests initially checked counts only; a cancelling
LICENSE/.c mutation motivated checking actual selected paths, and an oracle A/B/A case
was strengthened to reject replacement by A/A.

Remaining broad survivors include ranking arithmetic, alternative context margins,
chunk-budget edges and derived metadata. They are not all proven equivalent. Exact
ranking quality and every resource edge are not established by unit/mutation coverage;
this remains experimental code. Artificial non-decreasing UTF-8/cache loops hit the
20-second mutation timeout and are not counted as assertion catches. Unviable generated
constructors/defaults are likewise separate. The final distinct-source regression
follow-up caught its targeted mutation plus four generated struct-field neighbors.

## Next decision

Keep the question library, source/index/inference seams and external-memory direction.
Do not add this baseline as a trusted answer UI. The next controlled development cohort
should improve source-role/authority retrieval (production versus tests versus historical
reports), supply less but more relevant context, and evaluate an answer-first/canonical
source-reference protocol. Separate development tuning from fresh held-out questions.
Compare supported sampling settings and compact contexts before assuming a larger/newer
model is required. Existing typed source views can show evidence independently of model
prose; this pilot does not establish reliable generated architectural relationships.
