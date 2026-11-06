# Matched BF16/Q4 pin-release readout

For contributors deciding whether higher precision resolves the pin-release error before
attempting a weight edit. **BF16 changes preferences but does not recover the original versus
guarded distinction in any of the four readouts.** Production readouts still prefer No for
both. Diagnostic prose flips to Yes for both, fixing the original preference while breaking
the guarded one. This does not support switching precision as a remedy or identify a tensor edit.

The [contract](../../docs/qa/BF16_PROBE.md) precedes inference. Four known development contexts,
two systems and two prefixes run twice per artifact: 32 one-token observations each, 64 total.
These are forced-prefix lexical readouts, not complete QA answers. All eight reserved
real-source cases remain unrun. The production model and response-format defaults stay unchanged.

## Matched inputs and artifacts

The cached BF16 intermediate and its Q4_K_M derivative come from the same pinned source
revision and converter in the [original build](results/weight-edit/build.json). BF16's
full-file hash matches that build record; Q4 matches the
[original comparison manifest](results/weight-edit/base/manifest.json). All 335 tensor
names/logical shapes and all GGUF metadata match
except `general.file_type`, including architecture, tokenizer and chat template. Tensor types,
payloads and storage order differ. BF16 includes some F32 tensors; this compares artifacts
and their CPU precision paths rather than isolating a hardware-independent quantization law.

Before any probability query, both servers reproduce all six candidate token IDs and all
16 historical preflight conditions exactly: source, messages, rendered/full prompt bytes,
token arrays and request bodies. A fresh Q4 control runs first and reproduces all 32
historical measurements exactly, including probabilities, censoring and margins. BF16
starts only after Q4 exits. Within each artifact, all 16 repeated measurements are identical.

The [artifact audit](results/bf16-probe/artifact-audit.json.gz),
[descriptors](library/token-bf16/models.json), preflights and
[integrity record](results/bf16-probe/integrity.json) retain the checks. No download,
conversion, new dependency, training or weight edit is included.

## What changes with precision

The margin is logP(Yes)-logP(No) for the declared token spellings. Positive favours Yes;
negative favours No. Each row below is a unique first-pass condition; pass two repeats it.
Original should say Yes and guarded should say No. Both tokens appear in every top-512
list, so these margins and their differences are exact for the selected token IDs.

| System / prefix | Original Q4 | Original BF16 | Guarded Q4 | Guarded BF16 | Original BF16 minus Q4 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Production / JSON | -0.179 | -0.242 | -0.526 | -0.546 | -0.063 |
| Production / prose | -0.580 | -0.751 | -0.865 | -0.969 | -0.171 |
| Diagnostic / JSON | -1.283 | -0.818 | -1.422 | -0.949 | +0.465 |
| Diagnostic / prose | -0.654 | +0.572 | -0.753 | +0.456 | +1.226 |

The diagnostic/prose shift is substantial, but the guarded margin also moves by +1.210.
Neither artifact separates the original/guarded pair into the expected global top tokens
in any of the four readouts. Quantization affects this measurement; it does not explain
the failure as a simple loss of an otherwise correct higher-precision decision.

Source sensitivity remains positive, with a larger effect from permuting the same original
source. Source shift below is original minus guarded; layout shift is permuted minus original.
The archived helper's `permutationShift` uses the opposite sign, original minus permuted.

| System / prefix | Source shift Q4 | Source shift BF16 | Layout shift Q4 | Layout shift BF16 |
| --- | ---: | ---: | ---: | ---: |
| Production / JSON | +0.347 | +0.304 | +0.897 | +1.088 |
| Production / prose | +0.285 | +0.219 | +0.946 | +0.811 |
| Diagnostic / JSON | +0.139 | +0.131 | +0.814 | +1.053 |
| Diagnostic / prose | +0.099 | +0.115 | +0.752 | +0.767 |

BF16 favours Yes for the permuted original under all four readouts, whereas Q4 does so
under three. That improvement on a known presentation does not establish stable source
reasoning. Whole captured contexts include snapshot metadata and different source lengths;
these contrasts do not isolate a semantic token, attention head or editable circuit.

For missing definitions, neither artifact makes Unknown the global top token in any readout.
Unknown is censored in eight production/prose entries per artifact; its log probability
stays null with an upper bound at the cutoff. No missing candidate receives an invented
zero, and the three choices are never renormalized. The candidate winner stays unresolved
when any choice is censored; global top and Yes/No margin remain separate.
Observed choice mass ranges from 0.447–0.568 to 0.536–0.667 in production and from
0.868–0.948 to 0.813–0.970 in diagnostic prompts, Q4 to BF16 respectively. These values
are not calibrated uncertainty or all possible expressions of abstention.

## Runtime and verification

Both use pinned b12110, CPU-only operation, two inference/prefill threads, nice 10,
context 8,192 and batches 512/128. Requests use explicit token arrays, n_predict=1,
n_probs=512, pre-sampling probabilities, temperature 0, seed 42 and cache_prompt=false.
No grammar, response format or logit bias is supplied. All 64 observations pass raw-response,
parser, evaluated-token-count and echoed-setting checks; cache_n is zero, prompt_n matches
the submitted 297–420 tokens and backend sampling is false throughout.

| Observation | Q4 | BF16 |
| --- | ---: | ---: |
| Peak process-tree PSS | 1.033 GiB | 1.745 GiB |
| Median one-token observation | 2.58 s | 4.49 s |
| p95 one-token observation | 3.00 s | 5.18 s |
| Server startup | 0.947 s | 0.948 s |
| Whole observed run | 82.9 s | 144.8 s |

BF16 costs about 1.74 times the median observation time and 0.71 GiB more peak PSS here.
These times mostly measure cold prefill, not complete-answer latency. Both fall below the
4-GiB observed target in this archived-context probe with no native repository index.
The observer enforces no RAM cap, samples every 250 ms and reports no sampling errors.
No per-condition memory peak, physical laptop/editor workload or statistical speed guarantee
is claimed. Correctness/mutation jobs finish before inference; both owned servers stop.

All 18 serial QA test files pass, including preflight mismatch, incomplete/extra-row and
censored-delta regressions. The focused test is observed failing before implementation.
Fresh formatting and all-target Clippy pass. Rust/Cargo inputs match d15a56d, so the passing
453-test/doctest [native baseline](results/bf16-probe/native-baseline.json) is retained,
not rerun; native source/build identities are checked.

Whole-module mutation testing catches **300 mutations with three reviewed equivalents**,
zero timeouts/errors/no-coverage. Raw score is 99.01 and the tool exits 1 at threshold 100;
equivalents remain survivors. The [exclusions](results/bf16-probe/mutation-equivalents.json)
explain the identical condition sets in either fixed pass, a redundant null check and
rejection at the following missing-row property access. CLI transport is outside mutation
scope; both real server preflights and 64 endpoint observations exercise it. The run uses
one low-priority worker. The [earlier mutation erratum](MUTATION_ERRATUM.md) remains applicable.

All 280 checksummed payloads across 13 historical archives verify unchanged. The
[new archive](results/bf16-probe/) preserves raw responses before parsing, complete top lists,
preflights, artifact audit/script, paired comparison, memory samples, validation logs and
[checksums](results/bf16-probe/checksums.json). The
[procedure](BF16_PROBE.md) describes reproduction.

This closes the matched precision control without selecting an edit or qualifying a QA
model. A later weight intervention would still need causal localization, matched semantic
controls and false-Yes/abstention regression checks before a frozen real-source evaluation.
