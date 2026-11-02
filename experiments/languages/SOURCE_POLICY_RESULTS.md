# Source-policy implementation evidence

Source selection now has a portable interface independent of analyzer registration.
Snapshots include capture-policy identity, including empty observations. A policy
change enforces full discovery before retaining unread documents. Optional C-family
scopes expose source-only candidates with provenance and explicit reasons; C/C++
syntax adapters remain the next implementation stage.

The [reference](../../docs/prototype/SOURCE_POLICY.md) defines configuration and
identity inputs. The [workspace guide](../../docs/how-to/run-workspace.md) documents
the repeatable scope, exclusion, and raw-language assignment options.

## Behavior and protocol

Discovery, hinted reads, and native event filtering share one policy. Tests demonstrate
scope expansion from an empty baseline, scope reduction, exclusion precedence,
path-component matching, normalized rule identity, most-specific assignments,
conflict rejection, `.c++`/`.h++`, case-sensitive hints, and dotfile handling.
Missing analyzers preserve selected source rather than removing it. Source-only files
create neither parsers nor fake persisted analyses.

Classification and provenance participate in document version v2; capture policy is
included in snapshot composition v3. Workspace/file identity remains stable, and raw
facts still use content/context cache keys. Pinned observations keep their original
metadata. A classification-only change can reuse another file's immutable document.

View and pinned-source responses use schema 2 for policy, classification, encoding,
and capture/analysis counts. Status/SSE envelopes remain schema 1. Browser checks select
headers, display source-only hover/inspector text, distinguish encoding from assignment,
observe live edits, and retrieve old pinned text/metadata. Raw outlines explicitly
describe unknown build-configuration coverage. The core model requires no filesystem
or source-control operations to apply selection rules.

## Correctness and mutation testing

Formatting, all **126 workspace Rust tests**, doctests, Clippy, and **32 Node/browser
checks** pass. New core/native and presentation tests first failed against the missing
interfaces/schema/UI behavior. Additional regression tests caught incorrect dotfile
classification before implementation was corrected.

| Final mutation scope | Tested | Caught/killed | Unviable | Timeout | Survivors |
| --- | --- | --- | --- | --- | --- |
| Nine complete Rust modules, core/native test packages | 290 | 228 | 59 | 3 | 0 |
| Controller, update adapter, source-description module | 367 | 351 | 0 | 16 | 0 |

The initial Rust run found missing CLI-option and coarse-event-filter checks. Tests
now cover explicit C assignments, help/port handling, and ignored non-source files.
One equivalent scope-validation mutation exposed duplicate validation; the redundant
pre-normalization checks were removed, leaving normalized-path validation responsible
for backslashes/traversal. The full rerun has no survivors and no new exclusions.
One existing equivalent directory-prefix comparison remains excluded.

The first presentation run found gaps for empty-selection diagnostics, multiple-outline
separators, and deduplicating UTF-8 diagnostics mixed with other warnings. Added tests
kill those mutations; the complete three-module rerun has no survivors or exclusions.

The three Rust timeouts suppress initial publication and leave startup tests waiting.
The 16 JavaScript timeouts occur in asynchronous controller/update handling; none is in
the new source-description module. Timeouts and unviable builds are separate from
mutations detected by failing assertions. [The compact mutation report](results/source-policy-mutation-2026-11-01.json)
records scope, commands, source fingerprints, and outcomes that were not caught by assertions.
Identity macro expansion, dependency internals, filesystem event delivery, canvas
painting, and application glue are outside mutation scope. Coverage is evidence,
not a correctness proof.

## Native regression measurements

These use the final optimized binary, four shared workers, warm OS caches, disposable
copies of the same pinned archives, and three samples per condition. No tests or
mutation jobs ran concurrently. Every observed edit read one file, parsed one file,
and matched a separate process's complete capture. C-family scopes are disabled in
these runs: they measure the default Rust/Python pipeline after adding policy metadata.

| Default capture scope | Files | Empty analysis cache | Fresh-process persisted cache | One-file engine attempt | Write to coherent diagnostic response |
| --- | --- | --- | --- | --- | --- |
| lpc-rs Rust/Python | 552 | 367.42–385.74 ms | 24.96–26.77 ms | 2.29–2.42 ms | 31.69–32.45 ms |
| PyTorch Python | 5,077 | 5,910.04–5,960.04 ms | 359.13–368.62 ms | 10.65–13.31 ms | 53.40–69.06 ms |

The settled complete-verification attempts took 14.98 ms and 170.78 ms respectively.
Live peak RSS was 34.2 MiB for lpc-rs and 404.7 MiB for PyTorch. These are process
measurements on the earlier i7-13700H / approximately 62.5 GiB host, not memory limits.

Compared with the [adapter baseline](RESULTS.md), Rust's one-file engine attempt now
takes approximately 2.3 ms instead of approximately 1.4 ms. Policy/classification
validation adds work; these small, unpaired samples do not isolate its cost. There is
no speedup claim. The result retains one-file verification and coherent publication.
PyTorch's selected scope remains usable without C++, CUDA, headers, or generated inputs
being misrepresented as compiler analysis.

[The native report](results/source-policy-native-2026-11-01.json) retains binary hashes,
source revisions/inventories, full measurements, and clean-capture checks. Both reports
are based on `a74e75a` with the implementation uncommitted. The initial follow-up command
is the same [native driver](../prototype/README.md); use new output paths.

## Remaining qualification

No kernel/C-family application performance, configured translation-unit analysis,
browser render latency, GPU performance, or 16 GB laptop qualification is established
by these native samples. The successful browser tests use a small mixed fixture.
Capture bytes and parser memory still have no process-wide memory budget; a long parse
is not interruptible. Raw C/C++ extraction and representative qualification follow the
[remaining plan](../../docs/plans/c-family.md).
