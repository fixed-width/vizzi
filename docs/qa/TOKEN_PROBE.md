# Pin-release next-token probe

For contributors testing whether a source distinction appears in a small model's next-token
preferences before selecting a weight edit. Freeze this protocol before probabilities are
observed. No weights, production output contract or reserved questions change.

Use the four existing pin cases from the captured evidence-use archive: unconditional
finish after error (Yes), success-guarded finish (No), permuted unconditional source (Yes),
and missing finish definition (Unknown). Preserve their questions, captured source and
authorship limits. These are known synthetic development cases, not held-out QA accuracy.

Compare two systems: the exact existing production system instruction and a fixed diagnostic
instruction defining Yes/No/Unknown. Preserve the user/source message byte-for-byte. The
diagnostic system contains no case-specific facts or correct-choice hints. It is a new
readout condition, not a production improvement or a pure reproduction of prior answers.

At each system/case, force either the JSON prefix {"answer":" or prose prefix Answer:.
Probe the specific standalone token spellings Yes/No/Unknown for JSON and leading-space
spellings for prose. Tokenize and detokenize each spelling before any probe; require one
distinct token per choice and exact round trip. If this fails, stop without substituting
new labels. Unknown is not requested by the original production instruction, so its
production probability cannot be treated as a calibrated abstention score.

Render via the pinned runtime with thinking off, append the declared prefix, tokenize
special chat markers with add_special=false, and submit that explicit token array to the
native /completion endpoint. Record token IDs and exact text. Input token arrays avoid
implicit BOS insertion. The prompt is tokenized before the answer; this measures the
declared next-token IDs, not full-concatenation tokenization or all spellings of a word.

Use n_predict=1, n_probs=512, post_sampling_probs=false, temperature 0, seed 42, no grammar,
no response_format or logit bias, and cache_prompt=false. The pinned implementation reports
pre-sampling log-softmax values normalized across the vocabulary. Retain the entire returned
top list. Do not renormalize the three candidates, force a choice, or treat an absent
candidate as zero probability. It is censored at the last returned log probability.

Report logP(Yes)-logP(No) when both are observed, or a one-sided bound when one is censored.
With both censored the margin is unknown. Compare original-minus-changed margins within
the same system/prefix; report both signs and changes, including cases where both remain
No-biased. Compare permutation sensitivity and the diagnostic Unknown preference separately.
The global top token may be outside these choices; retain its identity and the probability
mass of observed choices so a forced-vocabulary winner is not called a complete answer.

Make two serial passes over 4 cases x 2 systems x 2 prefixes, reversing unique-condition
order for pass two: 32 one-token observations. Use one unmodified cached 0.8B Q4 model,
b12110, CPU-only, two inference/prefill threads, nice 10, context 8,192, batches 512/128,
and 360-second timeout. Disable prompt caching for this probe rather than reuse earlier
generation timings. Stop owned processes afterward. Finish correctness/mutation/build
work before measurement; no inspected project tooling or new dependency is needed.

Validate one probability position, requested top-list size, unique IDs, finite ordered
log probabilities, pre-sampling settings and exact evaluated prompt-token count. Retain
failures and censored entries; do not change the top-list size after seeing the results.
These conditional readouts are not complete answers, calibrated confidence, causal
activation evidence or a located neuron. No custom edit or reserved evaluation follows
automatically. Future qualification still needs fresh/count-matched controls.
