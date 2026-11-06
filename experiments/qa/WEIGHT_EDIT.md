# Compare the published weight edit on CPU QA

For contributors repeating the paired 0.8B experiment. Follow the
[contract](../../docs/qa/WEIGHT_EDIT.md) and use fresh output directories. The
[model manifest](library/weight-edit/models.json) describes locally built GGUFs; its
repository revisions identify the source checkpoint/patch rather than downloadable GGUFs.
The historical third-party 0.8B artifact is not the control for this comparison.
Read the [completed results](WEIGHT_EDIT_RESULTS.md) before choosing another condition.

## Reconstruct and audit the pair

Download the base checkpoint at `2fc06364715b967f1860aea9cf38778875588b17` using
`hf download Qwen/Qwen3.5-0.8B --revision ... --max-workers 1 --local-dir ...`.
Include the safetensors shard/index, configuration, tokenizer files, template, vocabulary,
merges, model card and licence. Verify the downloaded files with `hf cache verify` at the
same revision. Missing unused image-preprocessing files are distinct from corrupt
downloaded files.

Fetch `apply_edit.py`, `st_numpy.py`, `exp8_math/runs/p50c/edit/manifest.json` and
`edit.npz` from the [authors' repository](https://github.com/RikiyaT/lm_understand-logs)
at `4d09ba6d6a7e891dd8063974dfe81b3d78e3494f`. Inspect the code before execution.
Copy the base configuration/tokenizer/index files to a separate edited directory.
Run the reconstruction script with the base shard, patch directory and an edited
shard output with the same filename as the base. It verifies the published base/edited
tensor hashes. Preserve the base file.

Use an isolated environment with CPU-only PyTorch. The recorded environment uses current
stable Python packages compatible with the pinned converter, including PyTorch
2.15.0+cpu and Transformers 5.22.2. Reproduction should use the archived package versions
and source hashes rather than resolve a new environment and assume identical artifacts.
llama.cpp's older exact development requirement pins were not used; both conversions and
the strict paired audits pass with the recorded versions. No application dependency changes.

Before conversion, audit the whole source pair:

```sh
python experiments/qa/weight_edit_audit.py source BASE_SHARD EDITED_SHARD \
  --manifest PATCH_DIR/manifest.json --output SOURCE_AUDIT.json
```

Both conversions use `convert_hf_to_gguf.py` from llama.cpp commit
`cd959bae99ff4bed22bef2604f45241c09bbb8c8`, `--outtype bf16` and the same explicit
`--model-name 'Vizzi Qwen3.5-0.8B paired diagnostic'`. Set offline mode, two OMP/BLAS
threads and `nice -n 10`. Do not export a vision projector or enable remote model code.
Quantize each fresh BF16 file with the same pinned b12110 binary:

```sh
nice -n 10 /path/to/llama-quantize BASE_BF16 BASE_Q4 Q4_K_M 2
nice -n 10 /path/to/llama-quantize EDITED_BF16 EDITED_Q4 Q4_K_M 2
```

Put the pinned converter's `gguf-py` directory on `PYTHONPATH`. Audit both the BF16 and
Q4 pairs with `weight_edit_audit.py gguf ...`, using the same `--manifest` argument and
fresh audit outputs. Require identical metadata, names, shapes/types and unrelated
payloads, with changes in exactly `blk.15.ffn_down.weight` and
`blk.19.ffn_down.weight`. If the edit disappears or unrelated data changes, stop before
inference and report the constraint. Check resulting sizes/hashes against the model
manifest. Preserve build commands, package versions and all hashes.

## Verify and measure

Run the normal Rust/Node checks and the Python artifact tests before inference:

```sh
PYTHONPATH=/path/to/pinned/llama.cpp/gguf-py \
  python experiments/qa/weight_edit_audit_test.py
```

These tests include real small GGUF files and CLI rejection of lost edits, unrelated
tensor changes and changed chat metadata. Rust is unchanged, so no new native mutation
cohort is required. Finish all downloads, conversions, builds and tests before timing.

Prepare all fifteen source-complete cases:

```sh
nice -n 10 node experiments/qa/packs.mjs prepare \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/weight-edit/models.json \
  --model qwen3.5-0.8b-base-local-q4 --field-order answer_first \
  --repo /path/to/lpc-rs --output /tmp/vizzi-qa-weight-edit-prepared
```

Run one variant at a time under its own observer. The models root contains
`base/model-q4.gguf` and `edited/model-q4.gguf`:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-weight-edit-base.memory.json -- \
  nice -n 10 node experiments/qa/packs.mjs run \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/weight-edit/models.json \
  --model qwen3.5-0.8b-base-local-q4 --field-order answer_first \
  --prepared /tmp/vizzi-qa-weight-edit-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-qa-weight-edit-base-run
```

Wait for shutdown before repeating for `qwen3.5-0.8b-edited-local-q4` with separate memory
and output paths. Both use non-thinking mode and unchanged response limits. Do not
silently switch to the article's thinking configuration after inspecting results.

Grade every answer with the corrected dispatch oracle and unchanged material-fact rubric.
The existing `model-compare.mjs` compares source/messages/settings and paired grades; use
`results/dispatch-erratum/decomposition-library.corrected.json` as its library. Keep
resource fit, semantic completeness and reference validity separate. Archive raw answers,
runtime logs, memory samples, preparation/build metadata, audits and verification logs.
