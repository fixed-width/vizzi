# Prepare the contextual-review task

Use this how-to to materialise the frozen local-change review fixtures for
[milestone 2](../../docs/plans/product-milestones.md). Git and Python 3.12+ are required.
The review interface now passes both guided rendered variants. This guide prepares
its frozen input and repeats the walkthrough; the [report](WALKTHROUGH_RESULTS.md)
records the evidence and its limits.
The [portable inventory validation](INVENTORY_RESULTS.md) records the first implemented
comparison-model slice, independently of this Git fixture preparation.
The [textual-diff validation](TEXT_DIFF_RESULTS.md) checks bounded hunks on the same inputs.

From the workspace root, using a local lpc-rs repository containing the pinned commit:

```sh
python3 experiments/review-workflow/materialize.py \
  --source ../lpc-rs --variant variant-a --output /tmp/vizzi-review-a
python3 experiments/review-workflow/verify.py --fixture /tmp/vizzi-review-a
python3 experiments/review-workflow/materialize.py \
  --source ../lpc-rs --variant variant-b --output /tmp/vizzi-review-b
python3 experiments/review-workflow/verify.py --fixture /tmp/vizzi-review-b
```

Choose new output directories. Each contains `workspace/`, a disposable Git repository,
and `manifest.json`. The builder archives the frozen source tree, checks its Git tree
identity, imports the exact baseline commit as a shallow root, then applies the frozen
staged/unstaged patches and adds an untracked Markdown document. It refuses existing
output directories. It does not change the source checkout, fetch, run lpc-rs, invoke
project scripts, or alter Vizzi's current language policy.

Successful verification reports two changed paths, working bytes different from the
index, and the expected hunk count. It checks baseline/index/working byte hashes,
stage status, complete changed-path inventory and frozen hunk coordinates. The
[preparation report](RESULTS.md) records the initial checks and their limits.

## Check the portable hunks

After preparing both fixtures, build the source-only diagnostic example and run:

```sh
cargo build -p vizzi-core --example text_diff --offline
python3 experiments/review-workflow/verify_hunks.py \
  --fixture /tmp/vizzi-review-a --output /tmp/vizzi-review-hunks-a
python3 experiments/review-workflow/verify_hunks.py \
  --fixture /tmp/vizzi-review-b --output /tmp/vizzi-review-hunks-b
```

Choose new output directories. The checker verifies preparation, reads raw pinned/index
objects with Git, and supplies before/after bytes to the example. It checks the Rust
working and staged hunk coordinates against frozen expectations, plus the added
Markdown hunk and absent base. Each output retains `base.rs`, `index.rs` and `hunks.json`.
No analysed-project script or build runs. This exercises the portable model without
implementing native Git acquisition or a review interface.

For another already chosen pair of files, the diagnostic accepts a logical path and
two input filenames; `-` means a verified absent side:

```sh
cargo run -p vizzi-core --example text_diff --offline -- \
  src/example.rs /tmp/before.rs /tmp/after.rs
```

It prints metadata and unified-style hunk coordinates, using default diff limits and
no language analysis. Its input capture is a diagnostic helper, separate from future
native source admission and workspace discovery.

## Check native Git capture

On Linux, after preparing both fixtures, build the source-only native diagnostic:

```sh
CARGO_BUILD_JOBS=1 cargo build -p vizzi --example review_capture --offline
python3 experiments/review-workflow/verify_native.py \
  --fixture /tmp/vizzi-review-a --output /tmp/vizzi-native-a.json --require-admission
python3 experiments/review-workflow/verify_native.py \
  --fixture /tmp/vizzi-review-b --output /tmp/vizzi-native-b.json --require-admission
```

Choose new output files outside the fixture workspaces. This checker uses the native
Git object/working adapters directly, then portable inventory comparison and hunks.
It checks changed paths/statuses, the selected Rust and added-document hunks, pinned
baseline and repeatable pair identity, and verifies fixture status/bytes afterwards.
Each report records the complete path counts and source-only coverage. No analyser,
analysed-project build, fetch, host publication or browser UI runs. See the
[native validation report](NATIVE_RESULTS.md) and [acquisition reference](../../docs/prototype/REVIEW_ACQUISITION.md).

For a chosen workspace/revision/file, the diagnostic invocation is:

```sh
cargo run -p vizzi --example review_capture --offline -- \
  /tmp/vizzi-review-a/workspace \
  1a4f5b65dd2a269298748df2dfd68092a169a8d9 src/compiler/mod.rs
```

Add `--analyze` to the diagnostic invocation to apply the engine's default raw
Rust/Python policy through the shared executor. It uses one worker and a disposable
external analysis cache, retains other text as source-only, and prints selected
declaration/context metadata. It never runs lpc-rs or enables C-family analysis by suffix.
To check the frozen variants with analysis:

```sh
python3 experiments/review-workflow/verify_native.py \
  --fixture /tmp/vizzi-review-a --output /tmp/vizzi-analyzed-a.json \
  --require-admission --require-analysis
python3 experiments/review-workflow/verify_native.py \
  --fixture /tmp/vizzi-review-b --output /tmp/vizzi-analyzed-b.json \
  --require-admission --require-analysis
```

The [analysis report](ANALYSIS_RESULTS.md) separates syntax/ownership checks from the
future linked-UI walkthrough and performance qualification.

It uses [bounded review ownership](../../docs/prototype/REVIEW_OWNERSHIP.md) for both
capture and query lifetime, including the shared source-byte allowance. Admission
checks are recorded in the [ownership report](OWNERSHIP_RESULTS.md). The
[analysis bridge](../../docs/prototype/REVIEW_ANALYSIS.md) now uses the shared engine.
The [comparison API](../../docs/prototype/COMPARISON_API.md) exposes host operations;
the initial [review interface](INTERFACE_RESULTS.md) is implemented with browser
acceptance still open.

## Run the guided rendered walkthrough

In an environment that permits a local listener and Chromium, run each prepared
variant separately with a fresh output directory whose parent already exists:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- \
  --review-walkthrough --fixture /tmp/vizzi-review-a --output /tmp/vizzi-rendered-a
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- \
  --review-walkthrough --fixture /tmp/vizzi-review-b --output /tmp/vizzi-rendered-b
```

The runner builds Vizzi, uses one native worker and launches the normal HTTP host.
It navigates rendered file/hunk/declaration controls, unchanged reader/path contracts,
the regression test, the added document, history and refresh/expiry recovery. Response
data audits source identity and bounds; it does not choose navigation targets.

Each output records `observations.json` and fixture verification before/after. If the
browser starts, it also retains screenshots, `walkthrough.webm` and `trace.zip`.
Source rows are checked against independently read pinned/working bytes. An unsuccessful
attempt records its failure phase with `completed:false`; those files are not acceptance
evidence. Use `--no-build` only to deliberately reuse the recorded release binary.

Inspect the screenshots and recording after a successful run. The scripted judgement
is reference-aware and does not replace an independent reviewer. Current results,
recordings, audit tests and earlier blocked attempts are in the
[walkthrough record](WALKTHROUGH_RESULTS.md).

## Measure focused Linux/PyTorch review views

Use the supplied pinned profiles in [scale-profiles.json](scale-profiles.json); edit
their `source` paths if needed. The originals must contain those commits. The
[method](SCALE_PLAN.md) defines the working-tree setup, one-worker policy and timing
endpoints; [results](SCALE_RESULTS.md) distinguish startup, interaction and explicit
refresh from earlier whole-repository qualification.
The measurement driver now verifies [deferred startup](../../docs/prototype/STARTUP_PRODUCT.md):
review leaves the live index unstarted. Earlier eager measurements are preserved in
`results/scale`; repeat into new output directories.
The [startup report](STARTUP_RESULTS.md) and `results/startup` retain the repeated cohort
and its correctness/mutation evidence. Use `summarize-startup.mjs` as documented there
to compare three-session outputs with the earlier eager cohort.
The [context-preservation report](REFRESH_RESULTS.md) and `results/refresh` retain the
next cohort, with explicit selected-file assertions and unchanged setup. Aggregate it
against the startup cohort using `summarize-refresh.mjs OUTPUT.json LINUX_RESULTS
PYTORCH_RESULTS`; output files must be new.

Select the profiles into temporary files:

```sh
python3 - <<'PY'
import json
from pathlib import Path
for profile in json.loads(Path("experiments/review-workflow/scale-profiles.json").read_text()):
    Path("/tmp/vizzi-scale-" + profile["name"] + ".json").write_text(json.dumps(profile))
PY
```

Build Vizzi, then prepare and measure each case sequentially. Choose fresh output
directories with an existing parent and enough space for the full working-tree copies.
No project tooling runs and the original repositories are not edited:

```sh
CARGO_BUILD_JOBS=1 cargo build --release --offline -p vizzi
for review_case in linux pytorch; do
  python3 -B experiments/review-workflow/scale_fixture.py \
    --profile "/tmp/vizzi-scale-$review_case.json" \
    --output "/tmp/vizzi-scale-$review_case-fixture" --probe "$review_case"
  CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 npm test -- \
    --review-scale --fixture "/tmp/vizzi-scale-$review_case-fixture" \
    --output "/tmp/vizzi-scale-$review_case-results" --sessions 3 --no-build
done
```

Run without competing tests, mutations or benchmarks. Each report retains raw durations,
frame intervals, successful resource timings/body sizes, source/version audits, coverage,
RSS/heap samples, errors and source-status checks. Screenshots accompany the three
sessions. Failed sessions retain their last UI state. Host startup and first comparison
are separate; disk edits remain pinned until explicit refresh. These focused, finite
observations do not qualify full-repository or baseline-laptop performance.

Check fixture preparation independently with
`python3 -B experiments/review-workflow/scale_fixture_test.py`.

## Run an independent evaluation

The [transport replay](TRANSPORT_RESULTS.md) now checks these fixtures through normal
HTTP/stdio handlers. It remains separate from the upcoming linked-browser walkthrough.

Give the operator only [the neutral task brief](fixtures/task.md) and one disposable
workspace. Select baseline `1a4f5b65dd2a269298748df2dfd68092a169a8d9` and repository-root
scope. Both variants include the untracked document as part of the working side.

Keep this guide, patch files, [oracle](fixtures/oracle.json), [evaluation notes](ORACLE.md)
and reports hidden until the operator has supplied a verdict and evidence. Use neutral
workspace names; counterbalance variant order across operators. After one variant is
explained, the other is no longer an unfamiliar task for that operator.

Record familiarity with the codebase, navigation/refresh actions, source evidence,
unanswered questions, incorrect conclusions and time waiting for views. A scripted
guided replay establishes functional behaviour; an unfamiliar-human study is separate.
Do not report fixture preparation as a successful Vizzi review walkthrough.
