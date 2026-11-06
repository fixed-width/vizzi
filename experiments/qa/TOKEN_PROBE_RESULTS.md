# Pin-release conditional token readout

For contributors deciding whether the repeated pin-release error suggests a weight edit.
**The readout shifts with the source distinction, but still favours No for the unconditional
original. Layout shifts are larger in this example.** No tensor edit or calibrated confidence
claim follows. The full-schema production default and model weights remain unchanged.

The [contract](../../docs/qa/TOKEN_PROBE.md) precedes probability observations. Four known
pin cases run at JSON/prose answer prefixes under the existing production system and a
diagnostic system defining Yes/No/Unknown. Two serial passes reverse condition order:
32 one-token observations, not 32 complete answers. All sixteen repeated measurements
are exactly identical, including candidate probabilities and margins.

## What the scores show

The margin is logP(Yes)-logP(No) for the declared token spellings. Positive favours Yes;
negative favours No. Values are vocabulary-normalized pre-sampling log probabilities,
not a three-choice renormalisation. Each row below is the unique first-pass condition;
the second pass reproduces it exactly. Source shift is original minus guarded margin.

| System / prefix | Original, should call | Guarded, should skip | Same original permuted | Source shift |
| --- | ---: | ---: | ---: | ---: |
| Production / JSON | -0.179 | -0.526 | +0.718 | +0.347 |
| Production / prose | -0.580 | -0.865 | +0.366 | +0.285 |
| Diagnostic / JSON | -1.283 | -1.422 | -0.468 | +0.139 |
| Diagnostic / prose | -0.654 | -0.753 | +0.098 | +0.099 |

Original and guarded source move the margin in the expected relative direction in all
four readouts, but both still prefer No. Adding the explicit diagnostic instruction does
not make the unconditional original favour Yes. This is source sensitivity under a fixed
lexical readout, not proof that the model understands the program or that a useful answer
can be decoded by the existing production interface.

Permuting the same original ranges moves Yes-minus-No by +0.752 to +0.946 nats, larger
than the +0.099 to +0.347 source contrast here. Production prefixes and diagnostic/prose
then favour Yes for the permuted original, while diagnostic/JSON still favours No. The
displayed effect is not a located attention head or a causal weight mechanism.

For the missing-definition case, the global top token is Yes or No in every readout,
never the selected Unknown token. Even the explicit diagnostic instruction does not
make that token rank first. This does not measure all possible expressions of uncertainty
or what a full continuation would say. Unknown is not explicitly requested in the original
production system, so its production probability is especially unsuitable as an abstention
confidence score.

## Representation, censoring and scope

Preflight tokenizes/detokenizes all six spellings before any probability query and confirms
distinct single-token round trips. JSON uses unspaced Yes/No/Unknown, prose uses leading
spaces. Token IDs are recorded in [preflight](results/token-probe/preflight.json.gz):
JSON 9175/2665/13394 and prose 7179/2233/21024 respectively. Cross-prefix absolute token
probabilities are not interchangeable semantic word probabilities.

Yes and No are observed in every returned top-512 list, so all displayed margins are
exact for those token IDs. Unknown is censored in the production/prose lists: eight
entries across four cases and two passes. Its log probability remains null with an upper
bound at the returned cutoff; it is never assigned zero probability. The candidate winner
is deliberately unresolved when any choice is censored, while the global top token remains
separate. Observed three-choice mass ranges from about 0.447 to 0.568 for production and
0.868 to 0.948 for diagnostic prompts, leaving other vocabulary mass visible.

The production user/source message is byte-identical under the diagnostic system, but
the diagnostic system instruction and forced answer prefixes are deliberate interventions.
The prose prefix is not the production JSON response path. Original/guarded captured
contexts also include their snapshot identifiers and different source lengths; this
contrast does not isolate a semantic code token from every metadata/position change.
The missing/full cases retain their presentation-count cue. These are known synthetic
development inputs; all eight real-source reservations remain unrun.

This rules out neither quantization loss nor a weak source feature, instruction/readout
bias or larger-model capacity. It provides no justified weight patch. A useful next
model-side control is the cached BF16 version of the same base: keep these readouts fixed
and measure whether Q4 quantization contributes before choosing parameter interventions.
Any later edit still needs fresh/count-matched evaluation and regression coverage.

## Runtime and verification

The [cohort](library/token-probe/cohort.json) uses the unchanged cached 0.8B Q4 base and
b12110, CPU-only, two inference/prefill threads, nice 10, context 8,192 and batches 512/128.
Native /completion receives explicit token arrays, n_predict=1, n_probs=512, temperature 0,
seed 42, post_sampling_probs=false and cache_prompt=false. No grammar, response format or
logit bias is used. The pinned [server reference](https://github.com/ggml-org/llama.cpp/blob/cd959bae99ff4bed22bef2604f45241c09bbb8c8/tools/server/README.md)
documents these facilities; the local pinned implementation computes the reported
pre-sampling softmax across vocabulary logits.

All 32 observations pass one-position, evaluated-token-count, echoed-setting, finite
probability, unique-ID and ordered-top-list checks. All timings.cache_n values are zero
and prompt_n equals the submitted token-array length, 297–420 tokens. This runtime's
tokens_cached field records the retained prompt total, not reused prefill; it is not
used as a cache-hit count. Backend sampling is false throughout.

Whole observed time is 77.8 seconds and median observation time 2.41 seconds, mostly cold
prefill. This is not complete-answer latency. Peak process-tree PSS is 1.013 GiB with no
sampling errors; there is no native repository index in this archived-context probe.
No per-condition peak, enforced RAM cap, physical laptop or editor workload is claimed.
The owned server closes after the run.

Seventeen serial Node test files pass, along with fresh formatting/all-target Clippy.
Production Rust/Cargo inputs are unchanged from d15a56d, so its passing 453-test/doctest
baseline is retained rather than rerun; the [baseline record](results/token-probe/native-baseline.json)
binds its log and native tree hash. New helper tests are observed failing before implementation.

Whole-helper mutation testing catches **262 mutations, with two reviewed equivalents,
zero timeouts/errors/no-coverage**. [The two survivors](results/token-probe/mutation-equivalents.json)
are a redundant null branch under the parser's finite/null invariant and an early missing-row
guard whose next property access still rejects the input. Raw score is 99.24, exit 1 at
threshold 100; equivalents remain survivors. Runner transport is outside mutation scope
and has the tokenizer/rendering plus 32 real endpoint checks. Jobs use one low-priority worker.

The audit also found a mistaken historical equivalence judgment. The
[extra-row erratum](MUTATION_ERRATUM.md) demonstrates two old count-guard survivors accepting
an extra unique graded row, while normal implementations reject it. Those were coverage
gaps, now covered by regressions; no recorded QA response, score or resource result changes.
Historical raw archives/checksums remain unchanged, with explicit superseding evidence.

[Artifacts](results/token-probe/) retain raw responses before parsing, complete top-512
lists, tokenized/rendered conditions, [source effects](results/token-probe/effects.json),
[summary](results/token-probe/summary.json), [integrity](results/token-probe/integrity.json),
[validation](results/token-probe/validation.json), mutation/audit evidence and
[checksums](results/token-probe/checksums.json). The [procedure](TOKEN_PROBE.md) describes
reproduction. No new dependency, training or weight edit is included.
