# Prepare the contextual-review task

Use this how-to to materialise the frozen local-change review fixtures for
[milestone 2](../../docs/plans/product-milestones.md). Git and Python 3.12+ are required.
The review interface itself is not implemented yet; this prepares its acceptance input.
The [portable inventory validation](INVENTORY_RESULTS.md) records the first implemented
comparison-model slice, independently of this Git fixture preparation.

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

## Run an evaluation after review support exists

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
