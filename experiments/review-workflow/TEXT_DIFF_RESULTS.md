# Bounded textual diff validation

The portable comparison now computes exact raw-line edits and contextual hunks from
held source documents. Its [reference](../../docs/prototype/TEXT_DIFF.md) defines
coordinates, identity, resource charging and failure states. This is a model-layer
delivery; native Git acquisition, pair admission and the review UI remain pending.

## Correctness evidence

The first range test failed against the unimplemented interface before implementation.
Sixteen public-interface tests now pass. They cover manually specified insertion,
deletion and replacement ranges; empty/absent inputs; Unicode, CRLF and final-newline
changes; context grouping/clipping after line-count shifts; version/path/policy-bound
hunk identity; and exact byte reconstruction. A separate dynamic-programming oracle
checks minimum edit distance for all 961 pairs of binary-alphabet line sequences up
to length four, plus repeated-line examples over a larger alphabet.

Further cases exercise inclusive byte/line/hunk/work limits, late limit refusal,
reserved-trace-cell charging, long-line comparison cost, direct large additions and
deletions, unavailable/nonregular/unretained source, binary versus invalid UTF-8,
verified backing IO/integrity errors, and cancellation during computation or reads.
No error produces a partial successful result.

The [recorded mutation run](results/text-diff-mutation.json) covers all generated
mutations in `text_diff.rs`, without configured exclusions, with an unmodified passing
baseline. It caught 129 mutations; nine were unviable, with zero survivors and zero
timeouts. An earlier run exposed a missing independent work-accounting assertion and
nearby-context test case. Equivalent coordinate predicates were removed: unchanged
lines between edits advance both coordinates together. The final run assesses the
simplified implementation. Unviable cases are not counted as tests catching a change.

All 331 all-features workspace tests, doctest checks, formatting and Clippy passed.
Mutation evidence covers generated patterns in this module, not every possible bug,
native acquisition, host admission or browser behaviour.

## Frozen review inputs

The diagnostic example received the exact prepared baseline, index and working bytes;
its [recorded results](results/text-diff-fixtures.json) match the independently frozen
oracle. Original lpc-rs source was not edited or executed.

| Input | Rust working hunks | Rust staged hunks | Added Markdown |
| --- | --- | --- | --- |
| Variant A | 3, including the unstaged guard change | 1 | One full added-file hunk; base absent. |
| Variant B | 2, preserving the lookup guard | 1 | One full added-file hunk; base absent. |

Both include the documentation-comment hunk at base line 117 and diagnostic hunk at
166. Variant A additionally includes the guard hunk at base line 156; its later working
coordinates shift by two lines. Each run remains well within the default work
allowance. Charged work is not a timing or process-memory measurement.

Use the [fixture how-to](README.md#check-the-portable-hunks) to reproduce these checks.
The diagnostic supplies bytes to the portable model and does not establish native
Git inventory semantics, a review walkthrough, human comprehension, renderer performance
or scale qualification. The next slice is the minimal native acquisition seam and Git
base/working capture, with source/pair admission before host exposure.
