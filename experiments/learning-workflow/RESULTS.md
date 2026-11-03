# The learning task works; returning to earlier evidence loses context

The existing UI exposes the source needed to explain lpc-rs's compiler-read policy.
A guided replay locates the entry point, reader contract, path/name ownership and
regression test without an additional graph or analyser.

The concrete gap is **return navigation**: returning from the reader contract to the
compiler file clears the selected function and resets source to line 1. Browser Back
leaves the application. This supports investigation navigation as the next candidate;
it does not establish that graphs are generally unhelpful.

The operator already knew the paths, declaration names and expected answer. This is
functional evidence from reference-aware automation, not an unfamiliar developer's
session. See the [procedure](README.md) and [recording](results/walkthrough.webm).
Harness preparation inspected the UI implementation for selectors; replay navigation
uses UI controls and rendered DOM, not internal application facts. This does not establish
the milestone's independent-operator criterion.

## Answer supported by the UI

Input: lpc-rs commit `1a4f5b65dd2a269298748df2dfd68092a169a8d9`, default Rust/Python
capture, no LPC/C-family analysis. The overview reports 552 files with analysis.

| Question | Displayed evidence | Explanation |
| --- | --- | --- |
| Where does compilation read source? | `src/compiler/mod.rs`, `compile_file`, lines 140–148. [Entry point](results/03-compile-entry.png). | The entry point calls `read_file`, handles the source warning and continues through `compile_source`. |
| Which reader supplies it? | `read_file`, lines 151–157; `src/compiler/source_reader.rs`, lines 17–44. [Read policy](results/04-read-policy.png), [contract](results/05-reader-contract.png). | The compiler chooses an injected reader or `DiskSourceReader`. The trait supplies raw bytes; the default implementation uses Tokio filesystem reads. |
| When does it retry? | `read_file`, lines 158–166. [Read policy](results/04-read-policy.png). | Only NotFound with a resolved extension other than c enters fallback, constructing another source identity with `with_extension("c")`. A second missing c-suffixed read reaches the error branch. |
| How is failure reported? | `read_file`, lines 167–172; `src/compiler/source.rs`, `CompilerSource` and `name`. [Identity](results/08-source-identity.png), [accessor](results/09-diagnostic-name.png). | Other failures return an error using the diagnostic name. Resolved read paths and diagnostic names are separate fields. Full Config/LpcPath normalisation was not traced. |
| Where is the contract checked? | `file_reads_stop_after_extension_fallback_and_preserve_other_io_errors`, lines 2012–2078. [Test](results/10-regression-test.png), [cases](results/11-read-error-cases.png). | The existing Rust test checks attempted paths and preservation of the underlying error. The analysed project's tests were read, not executed. |

The displayed test specifies:

| Input/failure | Expected compiler calls to the reader |
| --- | --- |
| Unsuffixed missing input, NotFound | Original path, then its c-suffixed path |
| Already c-suffixed missing input, NotFound | One attempt |
| Unsuffixed input, PermissionDenied | One attempt |

These explanations come from reading Rust source and tests. Vizzi has not inferred a
resolved call graph or automatically validated the compiler's runtime behaviour.

## Observed navigation and its limits

All task checkpoints retain one immutable observation. Independent checks match
**371 visible row occurrences, representing 264 distinct reference-file rows**, to
the archive. The response audit has 39 bindings to that observation. Page-script
and response-collection errors are absent.

| Observation | Evidence | Interpretation |
| --- | --- | --- |
| Finding the starting point is manual. | Initially `benches/bench_arithmetic.rs` is selected; the script filters to `src/compiler/` and chooses the known module. [Initial view](results/00-initial.png). | The map provides file/directory orientation. Prior knowledge supplies the starting point here; novice discovery was not tested. |
| Search operates on paths. | `compile_file` yields zero matching files and retains the previous source. [Search probe](results/01-symbol-in-path-filter.png). | The labelled file filter behaves consistently. Workspace declaration lookup is absent, not broken. |
| The outline reaches the required code. | `compile_file` and `read_file` occupy positions 22/23; the regression test is position 127. Home/PageDown/End and selection reveal them. | Useful structural navigation exists. Scans restart deliberately, so counts are not a minimum route. Long labels retain full accessible text/title despite visual truncation. |
| Related source requires file choice. | Source identifiers are text; the script filters to the known reader/identity paths. | No import/definition navigation is exercised. Known paths suffice; ease of finding unknown targets is untested. |
| Returning loses the reading anchor. | Before leaving: `read_file`, scrollTop 3234, visible lines 148–184. After selecting the file again: no declaration, scrollTop 0, lines 1–37. [Before](results/04-read-policy.png), [return](results/07-manual-return.png). | The snapshot and bytes are unchanged. File selection behaves as designed; a separate return-to-location path is missing. This is a navigation gap, not an indexing failure. |
| Back does not revisit source. | URL/history length remain unchanged through file/declaration navigation. Back returns to `about:blank` in the fresh tab. [Back probe](results/12-browser-back.png). | The previous page depends on browser history. Vizzi currently supplies no source-location history through Back. |

## Next product decision

Prioritise a **bounded investigation navigation trail** using existing identities,
filters and reading positions. Restore prior locations when their source/version is
available, and make expiry or changed/ambiguous anchors explicit. Specify behaviour
and ownership before implementation; do not add unbounded snapshot pinning.

This addresses the directly observed return-to-evidence gap. Workspace declaration
lookup is a separate candidate for reducing path-based searching. An import graph
requires new facts and ambiguity handling; this replay does not justify making it the
next prerequisite. The [research rationale](../../docs/explanation/task-driven-views.md)
supports evaluating context preservation without assuming a visual encoding wins.

Step 1 of the [milestone plan](../../docs/plans/product-milestones.md) is delivered.
Milestone 1 remains open because returning to evidence does not preserve context.
Git-based review remains planned; no comparison UI or review fixture was built.

## Verification scope

Chromium 156.0.8078.4 runs at 1440×1000 with desktop-bus isolation and local-only HTTP.
The runner builds Vizzi and uses an exact source archive plus a fresh analysis cache.
Original source remains unchanged. Host readiness is approximately 335 ms; the first
source UI appears approximately 98 ms after page navigation. These single-run observations
use warm OS caches and scripted pacing; they do not qualify performance or human task time.

The application implementation is unchanged. The replay/dispatch route is checked by
this successful run, independent row/hash checks and **59 passing Node/browser checks**.
It does not extend native parsing/model mutation evidence. No source edit, stale-anchor
recovery or live-change usability scenario was exercised.

[Raw observations](results/observations.json), [summary](results/summary.json) and
[source/artifact hashes](results/source-sha256.json) retain the evidence. Full traces
are generated by reproduction; the committed bundle has the unedited video and checkpoints.
