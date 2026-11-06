# Run the source-complete diagnostic

For contributors checking whether the fixed CPU model can answer the three known
maintainer questions when relevant source is supplied. The [contract](../../docs/qa/SOURCE_PACKS.md)
defines the conditions. This diagnostic supplies literal repository source, not gold
answers, and does not measure a deployable retriever.

Run the [baseline checks](README.md) plus `node experiments/qa/packs.test.mjs`.
Build the native binaries with one compiler task:

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release \
  -p vizzi-qa -p vizzi-perf --bin vizzi-qa --bin vizzi-envelope
```

The [pack manifest](library/source-packs/packs.json) contains pinned full-file SHA-256
and BLAKE3 identities and inclusive source-line selections. Review complete definitions,
callers and branches before preparing a new pack. Keep evaluator notes outside prompts.
The commit pack covers the runner, task commit/deliver methods, channel helper, committer
commit arm, validation and publication. It excludes unrelated committer protocol arms.

Prepare a fresh Git archive and verify its identity against the original source capture:

```sh
node experiments/qa/packs.mjs prepare --repo /path/to/lpc-rs \
  --output /tmp/vizzi-qa-packs-prepared
```

Preparation validates source hashes, converts source lines to exact UTF-8 byte ranges,
checks the native selection against those bytes and audits required source anchors and
caller linkage. It records a manual coverage review and freezes code/data/binary hashes.
An oversized, stale or incomplete pack fails; it is never shortened automatically.

Finish compiler, test and serial mutation jobs before timing. Use fresh output paths:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-packs.memory.json -- \
  nice -n 10 node experiments/qa/packs.mjs run \
  --prepared /tmp/vizzi-qa-packs-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-models \
  --output /tmp/vizzi-qa-packs-run
```

The runner checks the fixed Qwen3.5-2B Q4 weights and llama.cpp runtime. It runs nine
responses on two CPU threads: compact retrieved evidence with 100 words/384 output
tokens; curated evidence with the same response allowance; and identical curated evidence
with 300 words/1,024 tokens. All conditions use up to eight ID citations, the same
2,000-character answer-schema cap, an 8,192-token model context and a 360-second timeout.
This raises the former four-citation cap uniformly; older runner defaults remain intact.
Arm order rotates by question and curated context hashes must match.

Grade the actual source claims, material facts, status and task completeness separately
from ID binding, using the unchanged maintainer rubrics. Record any ambiguity in notes.
The [assessment command](COMPACT.md) accepts the maintainer library via `--library`.
Keep these known-question diagnostics separate from held-out results. Check context
overflow, timeout, truncation and the unchanged character cap before attributing an
incomplete answer to model reasoning. Larger relevant contexts also cost more prefill
time; this experiment does not isolate relevance from context size.

Earlier frozen results remain unchanged. To reproduce their original executable/input
hashes, check out their measured implementation (`9c7789a` for the compact/maintainer
cohorts). The source-pack native extension changes the corpus version identity and
requires its own preparation; do not reuse an older prepared binary hash.
