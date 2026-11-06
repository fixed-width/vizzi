# Measure fixed-weight prefill profiles

For contributors screening memory use before repeating the full QA suite. The
[contract](../../docs/qa/PREFILL.md) defines the staged procedure. This experiment changes
runtime physical batch size, not weights or model precision. For weight-level alternatives,
read the [compression explanation](../../docs/explanation/model-weight-optimisation.md).
The [completed screen](PREFILL_RESULTS.md) finds no profile below the memory target and
therefore stops before full-suite validation.

The [frozen profiles](library/prefill/profiles.json) retain logical batch 512 and vary
physical microbatch 128, 32 and 8. Run normal baseline checks plus
`node experiments/qa/runtime-profile.test.mjs`. The model adapter tests use a temporary
stub runtime to verify the actual CPU-only child arguments and invalid-setting rejection.
No Rust/dependency changes or new native mutation run are required.

Prepare the source/code/data freeze for the resource screen:

```sh
node experiments/qa/packs.mjs prepare \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/model-comparison/models.json \
  --field-order answer_first --profiles experiments/qa/library/prefill/profiles.json \
  --cases decomp.commit-checks.full --repo /path/to/lpc-rs \
  --output /tmp/vizzi-qa-prefill-screen-prepared
```

After all correctness/build jobs finish, run one profile at a time with fresh output and
memory paths. Example for the smaller physical batch:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-prefill-micro32.memory.json -- \
  nice -n 10 node experiments/qa/packs.mjs run \
  --suite experiments/qa/library/decomposition/cases.json \
  --models experiments/qa/library/model-comparison/models.json \
  --field-order answer_first --model qwen3.5-4b-q4 \
  --profiles experiments/qa/library/prefill/profiles.json --profile micro32 \
  --cases decomp.commit-checks.full \
  --prepared /tmp/vizzi-qa-prefill-screen-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-prefill-micro32
```

Repeat for `baseline` and `micro8`, closing each server before the next. Record observed
process-tree PSS, actual arguments, initialization, source/message hashes, normal completion,
latency and any constraints. The timeout is uniformly 600 seconds. Do not select by answer
content. `selectResourceProfile` in `runtime-profile.mjs` implements the declared resource
rule: fastest successful full response among profiles at or below 4 GiB observed PSS.
If none fit, preserve the budget miss and stop the cohort.

When a profile qualifies, prepare again **without** `--cases` for all fifteen questions.
Use the same selected profile on the full run. A preparation records exact selected case
IDs and permitted profile limits; runtime case/profile/config mismatch fails. Compare
the full results against the corrected prior 4B grades from the
[model comparison](MODEL_COMPARISON_RESULTS.md). Reuse grades only for verified identical
source, prompt, raw output and native result; review every changed response.

Report resource-screen and full-suite counts separately. Identical weights do not promise
identical output under a different batching schedule. Record changed factual content,
false claims, statuses and source support rather than treating byte differences alone
as quality loss. Keep cold screen timing separate from prefix-cached full-suite timing.
No cap is enforced and the host is larger than the baseline device, so observed fit is
not a device qualification. Servers close after each measurement; no viewer/editor opens.
