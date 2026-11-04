# Portable textual diff reference

`vizzi_core::text_diff` computes complete raw-line edits and contextual hunks from a
held `Comparison`. It extends the [inventory interface](COMPARISON_TECH.md) without
native source acquisition, project tooling, a transport or a renderer.

## Input and result

`Comparison::text_diff(path, limits, cancelled)` inspects the requested inventory path
on both immutable sides. A verified absent side supplies an empty input; a retained
empty document remains present. Both absent produces `UnknownPath`. Known but
unretained source, unavailable presence/content and nonregular entries have distinct,
side-specific failures. Symlink targets and external-tree references remain inventory
metadata rather than fabricated regular text.

Combined captured byte lengths are admitted before loading either body. Captured
UTF-8 encoding is required. Reads use existing verified `CapturedSource` access;
backing IO, short-read and integrity errors propagate with their side. NUL-containing
input reports `Binary`, separately from invalid UTF-8. A failed read or unsupported
text never produces a successful empty diff.

| Result field | Meaning |
| --- | --- |
| `comparison`, `path` | Ordered observation pair and lossless logical path. |
| `base`, `working` | Optional document version, byte length, physical line count and final-LF flag. A missing version means verified absence. |
| `edits` | Ordered, non-overlapping contiguous changed runs; each has base/working byte and line ranges. Replacing each base range with its working range reconstructs the exact working bytes. |
| `hunks` | Grouped edits with up to three unchanged context lines on each side, base/working ranges and exclusive indices into `edits`. |
| `work` | Charged algorithm units, not elapsed time or a heap measurement. |

Ranges are zero-based and exclusive. LF terminates a physical line; its original
terminator bytes remain part of the line. CRLF is preserved, lone CR is ordinary
content, and no Unicode or whitespace normalisation occurs. Empty text has zero
physical lines. Text ending in LF has no additional virtual line in diff coordinates;
the [source inspector](WINDOWS.md) can still display that final empty editor row.
Zero-width ranges anchor insertions/deletions, including at EOF. Final-newline changes
are ordinary byte-significant line replacements; the per-side final-LF flags support
an explicit missing-newline presentation.

Edits separated by at most six unchanged lines share one contextual hunk. Seven or
more unchanged lines produce separate hunks. Line/byte offsets on each side remain
independent when an earlier edit changes line counts. Changes outside declarations
need no lexical match. Executable-only changes remain visible in the inventory even
when their textual result has no hunks.

Hunk identities use `raw-line-myers-v1`, the comparison identity, path, all four
allowances, grouped coordinates and edit indices. Repeating the same request is
deterministic; changing side order, source pair, path or allowances cannot reuse its
hunk identity. Result metadata does not own source bodies or publication leases.
Host admission and publication tokens remain planned and must wrap this content model.

## Algorithm and resource semantics

The implementation follows the shortest-edit-path method from
[Myers (1986), section 3](https://publications.mpi-cbg.de/Myers_1986_6330.pdf).
It searches by increasing insertion/deletion count and retains the furthest base
position for each reachable diagonal. Exact line-byte equality extends each path;
equal predecessor frontier positions choose a base advance. Backtracking produces the
shortest raw-line edit script, then groups context. No indent, semantic or rename
heuristic is applied. One-sided nonempty inputs use a direct single edit.

`DiffLimits::default()` permits 8 MiB of combined bytes, 200,000 combined physical
lines, 10,000 contextual hunks and 10,000,000 work units. Callers can supply explicit
allowances, including zero. These are inclusive maxima: a computation fitting exactly
may succeed. Exceeding one returns `Limit(Bytes|Lines|Hunks|Work)`, discarding every
partial edit/hunk. An empty textual result does not erase the inventory's added/removed
status for an empty file.

The work allowance charges:

- one unit per reserved frontier cell, prepaid for each complete row, including unused
  cells in the final partially visited row;
- one unit per candidate line comparison, plus the line's byte length when lengths
  match, before comparing those bytes;
- one unit per backtracked edit step and per traversed equal-line step;
- one unit per contextual-hunk start and per subsequent edit inspected for grouping.

Line indexing and byte loading are bounded by the separate input allowances. A failed
charge stops before the associated operation/allocation. Only the reachable diagonal
parity is retained. This is a trace-based implementation, not the linear-space
refinement: requested trace cells are bounded by `max_work`, so their requested storage
is at most 80,000,000 bytes on a 64-bit target at the default work allowance. Row/vector
and allocator overhead, loaded backing bytes, line indices, edits and result metadata
are separate. This bound does not qualify process RSS or concurrent host memory use.

Cancellation is checked before reads, while indexing lines, when charging frontier,
comparison/backtracking/output work, and immediately before returning a complete
result. A single backing read and bounded byte scan are not preempted. Cancellation
has its own error and no partial result. This is cooperative cancellation, not a
wall-clock deadline. Source/pair admission and scheduling are still required before
exposing capture or diff requests to a browser.

## Verification

The [validation report](../../experiments/review-workflow/TEXT_DIFF_RESULTS.md) records
TDD, independent byte reconstruction and minimum-distance checks, boundary failures,
mutation results and the frozen good/bad review inputs. The diagnostic
`text_diff` example supplies already chosen input files to this portable model; it
does not implement Git discovery, review publication, source windows or a review UI.
