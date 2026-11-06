# Matched BF16/Q4 readout control

For contributors checking whether Q4 quantization contributes to the pin-release error
before editing weights. Freeze this contract before new probabilities. Use the existing
cached BF16 GGUF intermediate and its locally quantized Q4_K_M derivative from the same
base checkpoint. No download, reconversion, training, tensor edit or dependency change.

Audit both full-file hashes against the original build record. Require identical tensor
names/logical shapes and all GGUF metadata except general.file_type, including tokenizer,
architecture and chat template. Storage order and tensor types/payloads may differ as
part of conversion to Q4. This is an artifact/CPU-kernel precision comparison, not a
hardware-independent quantization law.

Reuse the exact four known pin contexts, production/diagnostic systems, JSON/prose prefixes,
six word spellings and two-pass reverse order: 32 one-token observations per artifact.
Run a fresh Q4 control first, then BF16, each under its own process-tree observer. Never
keep both model servers resident. Require actual preflight candidate IDs, messages,
rendered/full prompt bytes, token arrays and request bodies to match the historical Q4
preflight before any query. Compare live-Q4 measurements with the historical control.

Keep b12110, CPU-only, two inference/prefill threads, nice 10, context 8,192, batches
512/128, temperature 0, seed 42, cache_prompt=false, n_predict=1, n_probs=512 and
post_sampling_probs=false. No grammar, response format or logit bias. Keep the 360-second
timeout and full-vocabulary-normalized top-list/censoring rules. Preserve failed conditions
without changing labels, top count, source or prompts. All eight reserved cases stay unrun.

Report Yes-minus-No margins and BF16-minus-Q4 deltas with bounds where censored, source
and permutation shifts, global top token, observed candidate mass and Unknown limitations.
Repeat measurements are not independent answers. Greater preference for an expected token
does not establish complete QA or identify an editable circuit. Diagnostic instructions,
prefix spelling and source/snapshot metadata remain deliberate inputs/confounds.

Record each cohort's observed PSS, startup, cold-prefill observation latency and failures.
Use the existing 4-GiB observed target without calling it a cap or device qualification.
If BF16 exceeds the target, retain that miss; do not silently quantize, resize context or
change concurrency. Stop owned servers after each run. Finish correctness/mutation work
before inference; unchanged native Rust may retain its verified 453-test baseline with
explicit source/build identity evidence. No inspected project tooling or viewer starts.

Use TDD for preflight comparison and paired readout scoring, including extra unique rows,
missing observations and censored deltas. Run focused whole-module mutation checks with
one low-priority worker. Keep prior archives, the mutation erratum and default model
selection unchanged. This control does not qualify a candidate or consume reservations.
