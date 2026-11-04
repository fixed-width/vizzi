# Native review acquisition reference

`vizzi::review_source` supplies immutable, source-only observations through a shared
`ReviewInput` list/read interface. Directory, pinned Git revision, Git working-tree and
in-memory inputs use the same capture function and portable comparison model.
Raw acquisition does not publish comparisons, run language analysis or add host/browser routes.
The [review ownership module](REVIEW_OWNERSHIP.md) now wraps it with admitted capture
and private native publication, separately from host/browser transport.
The [comparison contract](COMPARISON_TECH.md) still governs those later operations.

## Identity, scope and captured values

Directory and Git inputs use the same canonical-root workspace identity as the
existing directory engine. Git object-store paths and revision identifiers do not
become workspace/file identities. Revision source is read directly from objects,
without an archive, checkout or temporary source root.

`CapturePolicy::scope` selects the root with `None`, or a literal relative subtree,
plus explicit relative content exclusions. Prefix matching respects path components;
`src` does not include `src-other`. Exclusion order/duplicates are normalised for
policy identity. Excluded candidates remain unobserved/excluded inventory entries,
without a content read. Native enumeration can still inspect their names/metadata.

`acquire` requires complete enumeration before returning an `Acquired` value. A list
failure, duplicate candidate, cancellation or exceeded allowance discards the private
candidate. Per-path IO gaps remain explicit. Verified absence omits that side's entry;
unknown presence is never converted into absence. Source and inventory are validated
together by the portable observation constructor.

Regular files retain verified `CapturedSource` and source-only `Document` values,
including unsupported encodings. `AnalysisNotRequested` explicitly distinguishes
this phase from missing analyser support. No Rust, Python, C/C++ or LPC parser runs;
there is no new worker pool or analysis-cache path. An eventual analysis bridge must
use the existing scheduler/admission and freeze the same analysis policy on both sides.
This capture cannot be substituted for a live analysed snapshot merely because paths match.

Symlink targets and canonical external-reference bytes are hashed for inventory and
retained separately in `Acquired::values`. They do not become regular source documents.
Non-UTF-8/backslash paths retain lossless inventory identity even when the existing
document model cannot represent them. Byte identity, text availability and analysis
availability remain separate concerns.

## Git baseline and working rules

`GitRepository::open` requires the exact worktree root. Linked worktrees can use Git's
normal control-file resolution; bare repositories and subdirectory roots fail explicitly.
`pin` resolves and peels the selected revision once, freezes the resolved object identity
and enumerates its raw tree. Later ref moves do not retarget this input. Modes and
path bytes come from [NUL-delimited tree output](https://git-scm.com/docs/git-ls-tree).
One owned [batch object reader](https://git-scm.com/docs/git-cat-file) serves the baseline's
blob reads; there is no process launch per blob.

Working discovery unions pinned baseline paths, all current index paths/stages, and
non-ignored untracked candidates from [Git's standard enumeration](https://git-scm.com/docs/git-ls-files).
It then reads observed disk bytes, independently of staged content, clean status,
assume-unchanged flags, size or timestamps. Staged deletions with restored disk bytes
remain present; staged edits reverted on disk can compare unchanged. Unmerged paths
retain an explicit notice and the actual file, including conflict markers.

Manual skip-worktree absence is unobserved/sparse. Configured sparse-checkout capture
is currently unsupported and fails as a whole. Partial/promisor repository configuration
is also refused, so capture does not depend on fetching missing objects. These states
must not appear as deletions or an empty comparison.

Commands disable fsmonitor, hooks, untracked-cache use, optional locks, replacement
objects and lazy fetching. Inherited `GIT_*` overrides are cleared before setting the
acquisition environment. Normal Git configuration still supplies standard untracked
ignore rules. Raw object reads do not run filters, textconv, external diff or archive
attribute transformations. Command stderr is not forwarded as source data; errors
identify the failed acquisition operation without dumping configuration.

Gitlinks capture the nested HEAD pointer when locally available, with a
`NestedContentsNotCaptured` notice. No nested sources are traversed or assessed for
dirty/clean content. Missing/deinitialised pointers are explicit gaps. This phase does
not claim recursive submodule coverage. Rename inference remains add/delete.

## Filesystem and resources

The initial filesystem implementation uses safe
[rustix 1.1.6](https://docs.rs/rustix/1.1.6/rustix/) descriptor-relative operations on
Unix. Each ancestor and regular-file open rejects symlinks; symlink targets are read
literally. Special entries are described without reading a FIFO/device as source.
Regular reads compare pre/post metadata, path identity and actual byte count; changed
observations are unobserved/changing. A root replacement fails capture. These checks
do not promise an atomic multi-file transaction.

Linux is verified. Non-Unix filesystem capture returns an explicit unsupported state;
Windows acquisition and broader platform qualification remain pending. Portable
comparison, text-diff and host-interface concepts retain their platform independence.
Unsafe code remains denied in this project; the OS calls use an existing safe wrapper.

| `CaptureLimits` default | Scope |
| --- | --- |
| 2 GiB content bytes | One private acquisition's verified content, including literal target/reference values. |
| 32 MiB per entry | Refused before retaining a larger blob/regular body. |
| 1,000,000 candidates/entries | Git candidate count or visited raw directory entries, including directories. |
| 128 MiB discovery bytes | One Git listing output or accumulated raw directory path names. |

Limits are inclusive. These raw per-input limits alone do not implement global owner
quota or pair admission; the [review ownership module](REVIEW_OWNERSHIP.md) now provides
that admitted capture path. Metadata, buffers, source indexes and allocator overhead are
separate. Oversized content currently refuses the candidate; streaming binary-only
digests and backing/spool integration are not delivered here.

IO uses bounded chunks, with additional fixed bounds for Git control/configuration
responses and object headers. Owned children are killed/reaped on abandonment/drop;
stderr cannot block an undrained pipe. Cancellation is checked between operations and
chunks and before returning the observation. Individual blocking IO is not preempted;
this is not a wall-clock deadline. Future host calls must run outside publication locks
and share existing scheduling capacity.

The [native validation report](../../experiments/review-workflow/NATIVE_RESULTS.md)
records behavioural tests, mutation evidence and both frozen review variants. The
diagnostic example is not a published review workspace or a usability demonstration.
