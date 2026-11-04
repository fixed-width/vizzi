# Review fixture evaluation reference

For evaluators; keep this separate from the [operator brief](fixtures/task.md).
All coordinates refer to lpc-rs baseline
`1a4f5b65dd2a269298748df2dfd68092a169a8d9` or the frozen variant, not the latest checkout.
The machine-readable [oracle](fixtures/oracle.json) fixes hashes, inventory and hunks.

## Expected verdicts

Both variants change the source-read diagnostic to retain the file name and underlying
I/O error, adding `(source lookup stopped)`. Both have a benign unstaged `compile_file`
documentation change and the same untracked Markdown document. The index contains
only the diagnostic change. The comparison must use working bytes, not staged bytes.

**Variant A has a regression.** Its unstaged `read_file` guard permits suffix fallback
for `PermissionDenied` as well as `NotFound`. For `/source-read/denied`, the baseline
stops after the first error. The variant can instead read `/source-read/denied.c` and
even succeed, violating the change's stated intent. The base-to-working Rust hunks
start at base lines 117, 156 and 166; the guard is the second hunk. Reading the staged
patch alone misses the defect.

**Variant B preserves lookup behaviour.** Its only Rust changes are the diagnostic
wording and documentation comment, at base lines 117 and 166. Rejecting both variants
as having the same lookup regression fails the control. An operator may raise another
specific concern; evaluate its evidence rather than requiring a fixed phrase.

The additional Markdown document is an added text path outside the current default
Rust/Python analysis scope. It must remain visible and inspectable without a fabricated
declaration outline. It is absent in the base, not an empty base file.

## Supporting evidence

At the frozen baseline:

| Path / declaration | Evidence |
| --- | --- |
| `src/compiler/mod.rs`, `Compiler::compile_file` (line 140) | Obtains identity/source through `read_file` before compiling. |
| Same file, `Compiler::read_file` (line 151) | Uses the injected reader or `DiskSourceReader`; retries only NotFound when the extension is not already c. |
| `src/compiler/source_reader.rs`, `SourceReader` | Reader returns raw bytes or an I/O error; the default implementation reads the disk path. |
| `src/compiler/source.rs`, `CompilerSource` | Input, resolved path and diagnostic name have distinct roles. |
| `src/compiler/mod.rs`, `file_reads_stop_after_extension_fallback_and_preserve_other_io_errors` (line 2012) | Specifies missing-unsuffixed => two reads; missing-c-suffixed => one; PermissionDenied => one, with the underlying error retained. |

The regression test's injected reader fails for the expected number of reads, then
returns success. The broadened permission branch would therefore violate the expected
error/result as well as the call count. This verdict follows source inspection; fixture
preparation does not execute that project test. Exact variant line numbers shift after
the guard edit, so review navigation must use pair-bound ranges/declarations.

Preparation validation does not cover binary/symlink/submodule cases or prove the
comparison implementation correct. Those require the independent tiny cases in the
[technical matrix](../../docs/prototype/COMPARISON_TECH.md#required-correctness-matrix).
