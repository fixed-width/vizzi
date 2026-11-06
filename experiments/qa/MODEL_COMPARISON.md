# Compare CPU model capacity on frozen source

For contributors repeating the fixed-source comparison after the
[decomposition/output-order diagnostic](DECOMPOSITION_RESULTS.md). The
[contract](../../docs/qa/MODEL_COMPARISON.md) defines controls and limitations. This compares
Qwen3.5-2B and 4B Q4_K_M in the same setup; it is not each model's optimal configuration
or a generation-only comparison.

The [model manifest](library/model-comparison/models.json) pins both artifacts, revisions,
file sizes and SHA-256 values. Read the [official 4B model card](https://huggingface.co/Qwen/Qwen3.5-4B)
for architecture and supported non-thinking mode. The published card recommends different
sampling/output settings for general use; this diagnostic deliberately preserves the
previous temperature, source and output limits. Compatibility with the pinned llama.cpp
runtime must also be observed on load, not inferred from the parameter count.

Acquire only the pinned quantized file, using one worker. Example for the new candidate:

```sh
HF_HOME=/tmp/vizzi-qa-hf HF_HUB_DISABLE_TELEMETRY=1 HF_HUB_DISABLE_XET=1 \
  nice -n 10 hf download bartowski/Qwen_Qwen3.5-4B-GGUF \
  Qwen_Qwen3.5-4B-Q4_K_M.gguf \
  --revision 4168f45a16a1290d65a4ec0fa312ae917a4c15d6 \
  --local-dir /tmp/vizzi-qa-models/4b --max-workers 1
```

Run formatting, workspace tests/doctests, Clippy and the serial Node QA suite, including
`model-config.test.mjs` and `model-compare.test.mjs`. No native logic/dependencies change;
retain the prior native selection/provenance mutation evidence rather than rerunning it.
Finish downloads and correctness jobs before measuring either model.

Prepare one new source/code freeze for answer-first only:

```sh
node experiments/qa/packs.mjs prepare \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/model-comparison/models.json \
  --field-order answer_first --repo /path/to/lpc-rs \
  --output /tmp/vizzi-qa-model-compare-prepared
```

Run each model separately under the process-tree observer. Use fresh result paths and
wait for the first process/server to close before starting the second:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-compare-2b.memory.json -- \
  nice -n 10 node experiments/qa/packs.mjs run \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/model-comparison/models.json \
  --field-order answer_first --model qwen3.5-2b-q4 \
  --prepared /tmp/vizzi-qa-model-compare-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-compare-2b
```

Repeat with model `qwen3.5-4b-q4` and distinct memory/output paths. Each invocation checks
model bytes, checksum, runtime hash and the shared source/code freeze. Older runner
defaults remain unchanged when these explicit model/order options are omitted.

Grade every response independently against the unchanged question facts. When a live
2B output matches the earlier raw response, record that reproduction before reusing its
grade; otherwise review the new response. Canonical available IDs alone do not prove
source support. Keep broad/narrow outcomes and unsupported claims separate.

```sh
node experiments/qa/model-compare.mjs \
  --baseline /tmp/vizzi-qa-compare-2b/answers.jsonl \
  --candidate /tmp/vizzi-qa-compare-4b/answers.jsonl \
  --library experiments/qa/results/dispatch-erratum/decomposition-library.corrected.json \
  --baseline-grades /path/to/2b-grades.json --candidate-grades /path/to/4b-grades.json \
  --baseline-memory /tmp/vizzi-qa-compare-2b.memory.json \
  --candidate-memory /tmp/vizzi-qa-compare-4b.memory.json \
  --output /tmp/vizzi-qa-model-comparison.json
```

The scorer rejects mismatched evidence, prompts, output order or response settings and
reports paired improvements/regressions. The 4 GiB process-tree PSS target is observed,
not enforced; report exceedances and any load/context/runtime failures explicitly.
Prefix-cached latency and a 64 GB host do not establish cold-cache or 16 GB-device results.
Known-source guided questions and agent grading are not independent product acceptance.

The [dispatch oracle erratum](DISPATCH_ERRATUM.md) corrects an evaluator error discovered
in this comparison: `process` wraps the real `dispatch` handler. Keep the original frozen
question suite for inference, since gold facts are not sent to the model; use the corrected
receiver library and explicitly revised grades for scoring. Preserve the as-recorded
baseline grades and reproduction evidence alongside the corrected ones.
