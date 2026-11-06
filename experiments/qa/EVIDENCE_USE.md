# Run the evidence-use development diagnostic

For contributors reproducing a narrow failure before considering a custom weight edit.
The [contract](../../docs/qa/EVIDENCE_USE.md) fixes the questions, splits and conditions.
The [library reference](library/evidence-use/README.md) describes the case variants.
The [completed diagnostic](EVIDENCE_USE_RESULTS.md) records repeatable errors and a
post-output wording ambiguity without changing the frozen inputs.

The study has sixteen synthetic development cases and eight reserved real-source cases.
Only development cases can be sent to the model by this runner. Reserved preparation
audits source and gold; it does not generate answers. No new model download is needed:
use the locally built base Q4 artifact from the [weight-edit comparison](WEIGHT_EDIT.md).

Run workspace formatting/tests/Clippy and the serial QA tests before model timing:

```sh
node --test --test-concurrency=1 experiments/qa/*.test.mjs
```

The Rust inputs under `fixtures/evidence-use/{a,b}` are safe Rust 2024 data fixtures,
not workspace implementation crates. Check their formatting/compilation separately;
do not execute inspected third-party project tooling. Production Rust is unchanged.

For mutation testing of the new validation/admission/scoring module:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node node_modules/@stryker-mutator/core/bin/stryker.js \
  run experiments/qa/evidence-stryker.config.json
```

The configuration uses one worker. Investigate every survivor. The raw 100% threshold
also counts equivalent diagnostic/early-guard mutations; the current result documents
those individually rather than counting them as caught. Do not treat a future threshold
failure as equivalent without reviewing its actual report. Runner orchestration is not
included in this mutation scope.

Prepare a fresh output directory. Supply local PyTorch source; Vizzi defaults to this
repository. Git exports read pinned objects, not working-tree contents:

```sh
nice -n 10 node experiments/qa/evidence.mjs prepare \
  --pytorch /path/to/pytorch --output /tmp/vizzi-qa-evidence-prepared
```

Preparation copies only the declared files, checks SHA-256 and native content identities,
and audits exact ranges and required anchors. `inputs.json` contains development
questions/contexts. `reserved.json` holds separate audited contexts with zero generations;
`audit.json` retains expected support labels/status for the evaluator. `prepared.json`
freezes both libraries, code/native binaries, source files and prepared artifacts.
The ordinary model input contains question/repository and selected source, never gold.

After downloads/builds/tests/mutation jobs finish, run the unchanged 0.8B control:

```sh
target/release/vizzi-envelope --observe-only /tmp/vizzi-qa-evidence.memory.json -- \
  nice -n 10 node experiments/qa/evidence.mjs run \
  --prepared /tmp/vizzi-qa-evidence-prepared/prepared.json \
  --runtime /path/to/llama-server --models-root /tmp/vizzi-qa-weight-edit/models \
  --output /tmp/vizzi-qa-evidence-run
```

The runner makes two serial passes over the sixteen development cases. It reselects and
compares each context with preparation, validates references through the native process,
and records raw output, status, order, latency, finish reason and errors. It has no
reserved inference mode, arbitrary case filter or model-selection override. Unknown CLI
options fail. Both the model and the two fixture indexes close after the run.

Create one manual grade per response with the existing material-fact format: facts,
unsupported claims, citation support, role confusion and notes. Preserve exact case IDs
and repeat numbers. `diagnoseEvidence` in `evidence-study.mjs` checks the complete ledger
and reports source-change tracking, label-permutation pairs, missing-source abstention
and failures repeated across cases/families. A correct uncertain answer requires more
than the insufficient-evidence status: it must avoid unsupported specific conclusions.

Archive source audits, both split libraries/hashes, raw development answers, grades,
resource observations and validation/mutation reports. Report reserved generations as
zero. Any later edit needs a separate development loop and a frozen candidate before
reserved evaluation; these development answers can never become fresh held-outs.
