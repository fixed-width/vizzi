# Compare BF16 and Q4 conditional readouts

For contributors checking whether quantization contributes to the pin-release error.
The [contract](../../docs/qa/BF16_PROBE.md) fixes the comparison before probabilities;
the [results](BF16_PROBE_RESULTS.md) record its outcome. This procedure reuses known
development contexts. It neither edits weights nor runs the reserved questions.

Use the cached BF16 intermediate and its Q4_K_M derivative from the
[original build](results/weight-edit/build.json), with the pinned b12110 CPU runtime.
The [model descriptors](library/token-bf16/models.json) give full-file hashes and sizes.
Under a common models root, place them at `base/model-bf16.gguf` and
`models/base/model-q4.gguf`. Do not substitute another conversion or tokenizer.

Audit all GGUF metadata and tensor names/logical shapes before inference. Only
`general.file_type` may differ among metadata fields. Tensor types, values and storage
order differ by precision. The archived audit includes the exact script and metadata
fingerprints; it uses the pinned converter's existing `gguf-py`, not a new dependency.

Run correctness checks and the whole-module mutation configuration in
[the token-probe procedure](TOKEN_PROBE.md) first. The configuration includes the
precision-pairing helper and both test files. Keep one low-priority mutation worker;
finish all tests, mutation work and builds before measuring inference.

Prepare both cohorts into fresh directories:

```sh
nice -n 10 node experiments/qa/token-probe.mjs prepare \
  --cohort experiments/qa/library/token-bf16/control.json \
  --output /tmp/vizzi-qa-bf16-control-prepared
nice -n 10 node experiments/qa/token-probe.mjs prepare \
  --cohort experiments/qa/library/token-bf16/bf16.json \
  --output /tmp/vizzi-qa-bf16-candidate-prepared
```

Preparation verifies the historical input/preflight archives, then freezes their hashes,
the cohort, protocol, helper code and process-tree observer. At runtime, all 16 preflight
conditions and six candidate spellings must match the archived Q4 reference before any
probability query. Matching includes messages, prompt bytes, token IDs and request bodies.
A mismatch aborts the run; preserve that failure rather than adapting the readout.

Run the fresh Q4 control, wait for it to finish, then run BF16:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-bf16-control.memory.json -- \
  nice -n 10 node experiments/qa/token-probe.mjs run \
  --cohort experiments/qa/library/token-bf16/control.json \
  --prepared /tmp/vizzi-qa-bf16-control-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit \
  --output /tmp/vizzi-qa-bf16-control-run
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-bf16-candidate.memory.json -- \
  nice -n 10 node experiments/qa/token-probe.mjs run \
  --cohort experiments/qa/library/token-bf16/bf16.json \
  --prepared /tmp/vizzi-qa-bf16-candidate-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit \
  --output /tmp/vizzi-qa-bf16-candidate-run
```

The runner closes its owned server on completion or error. Never load both models
together. Keep two inference/prefill threads, CPU-only operation, context 8,192 and batches
512/128. Each cohort makes 32 cold-prefill one-token observations. A 4-GiB observed target
is a reporting threshold, not an enforced memory cap or laptop qualification.

Preserve raw responses, full top-512 lists and failure records. Pass each run's `model`,
`preflight` and parsed `probes.jsonl` rows to `comparePrecisionProbes` in
[the helper](precision-probe-support.mjs). It rejects incomplete or extra rows and compares
BF16-minus-Q4 Yes-minus-No margins, retaining bounds for censored values. Compare the
fresh Q4 measurements with the historical archive separately. Repeats, forced prefixes
and token probabilities do not establish full-answer accuracy or calibrated confidence.
