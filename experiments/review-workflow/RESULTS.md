# Contextual-review fixture preparation

On 2026-11-03, both frozen variants were materialised and independently checked with
Git 2.43.0 and Python 3.12.3. This report records acceptance-input preparation, before
comparison implementation. [Recorded checks](results/preparation.json) bind the oracle
hash and source pin; the [how-to](README.md) reproduces the positive checks.

| Check | Result |
| --- | --- |
| Exact baseline commit/tree, shallow repository integrity | Passed for both variants. |
| Baseline, staged, working and untracked-document hashes | Passed for both variants. |
| Complete changed inventory | Two paths each: modified Rust and untracked Markdown. |
| Staged/unstaged separation | Rust status MM in both; working bytes differ from the index. |
| Frozen base/working Rust hunk coordinates | Three in variant A, two in variant B. |
| Existing output directory | Refused; positive fixture still verifies afterwards. |
| Output inside the original checkout | Refused before creating any path. |
| Unexpected working source bytes | Rejected by frozen source hash. |
| Index bytes substituted for working bytes | Rejected by required stage/working state. |
| Untracked document omitted | Rejected by required inventory/state. |
| Original lpc-rs checkout | Clean before/after; only disposable copies were changed. |

Negative probes changed/restored files only under the newly created `/tmp` outputs;
both positive fixtures were reverified afterwards. Vizzi workspace tests, formatting
and Clippy passed. No production source changed, so no Rust mutation or browser run
is claimed here. No lpc-rs build or project test ran.
The expected behavioural verdict derives from the pinned source and existing regression
contract, as documented in the [evaluation reference](ORACLE.md).

This is not a Vizzi review demonstration, performance qualification or usability study.
The next delivery is the portable comparison model, followed by acquisition and linked
views that can actually exercise these inputs.
