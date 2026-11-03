# Product milestones: understand a subsystem, then review a change

Status: baseline learning walkthrough delivered; milestone 1 navigation acceptance and
milestone 2 remain open. This follows the product-alignment checkpoint at commit 4e75d04.

The goal is coordinated views for learning unfamiliar code and reviewing changes, as
defined in [DESIGN.md](../../DESIGN.md). The [research rationale](../explanation/task-driven-views.md)
explains the candidate perspectives. This plan governs the next delivery order; the
remaining questions in performance reports are not an automatic implementation queue.

## One real subsystem, two user tasks

Use the Rust implementation of lpc-rs, pinned for these tasks at
`1a4f5b65dd2a269298748df2dfd68092a169a8d9`. This is a workflow-fixture pin, separate
from earlier benchmark archives. Original checkouts remain unchanged. No LPC analysis,
project build or project script is required.

**Learning question:** How does the compiler obtain source, decide whether to retry a
path with the c extension, and report a read failure?

Reference evidence, checked by source inspection at that pin:

- `src/compiler/mod.rs`: `Compiler::compile_file`, `Compiler::read_file`, and the test
  `file_reads_stop_after_extension_fallback_and_preserve_other_io_errors`.
- `src/compiler/source_reader.rs`: the `SourceReader` contract and `DiskSourceReader`.
- `src/compiler/source.rs`: `CompilerSource` and its path/name distinctions.

The expected explanation connects the compile entry point, injected/default reader,
path identity and failure branch. A missing unsuffixed input permits a second read with
the c extension; an already-suffixed missing input and a permission error stop after one
read. Existing tests specify these cases. These answers are an oracle for the walkthrough,
not claims that Vizzi has already helped someone discover them.

**Review question:** Does a proposed diagnostic improvement preserve those source-lookup
rules, and what evidence supports the review?

Prepare a controlled patch on a disposable copy of that pin. Its stated intent improves
error wording; its seeded defect also permits suffix fallback after PermissionDenied.
The reviewer should find that behavioural change, navigate to the reader contract and
existing regression test, and explain the violated rule. Keep a correct variant as a
control so flagging every patch cannot pass. Freeze the patches and expected hunks before
implementation; do not edit the real lpc-rs checkout.
Keep expected answers and the seeded defect out of an evaluation operator's task brief.

## Milestone 1: an evidenced subsystem explanation

Begin with a recorded walkthrough using current views: launch the workspace, locate the
compile entry point, find the reader/path contracts and test, and assemble the explanation.
Record missing navigation, lost context and unsupported questions. Source inspection for
this plan is not that browser walkthrough.

The [baseline replay](../../experiments/learning-workflow/RESULTS.md) answers the question
using existing views, but returning to an earlier file loses its declaration/reading
position. The next candidate is a bounded session-local navigation trail using existing
identities. Workspace declaration lookup is secondary; the originally proposed focused
Rust module/import view remains deferred until a task requires it. Preserve keyboard
access and explicit bounds. Existing CLI launch is sufficient for this milestone.

Acceptance:

1. A walkthrough answers the learning question with anchors in the three reference files
   and the regression test, without inspecting Vizzi's implementation to navigate.
2. Selection is coordinated across existing views and any added structural view.
   Returning to earlier evidence preserves useful reading context.
3. Any structural view gives every relationship a kind and source evidence. Unresolved/external targets and
   unavailable relationship extraction are visible; raw imports never imply resolved calls.
4. Editing/moving a declaration cannot silently bind an old anchor to different source.
   Version expiry and ambiguity have explicit recovery states.
5. New relationship extraction is checked against duplicate names, grouped/aliased imports,
   conditional or unsupported module paths, missing targets and a mixed-language boundary.
   Unsupported cases stay explicit; capabilities not introduced remain deferred.

Deliver a reproducible task script, expected evidence and a recording/screenshots of the
walkthrough, including unresolved friction. Functional acceptance does not claim a measured
comprehension improvement. Record operator familiarity; human usability evaluation is
separate from agent/maintainer replay.

The recorded baseline is guided, reference-aware automation. Its successful source
explanation is functional evidence, not independent-operator acceptance or a comprehension
study. Milestone 1 remains open because return navigation does not preserve context.

## Milestone 2: a contextual local-change review

Select an explicit Git baseline and compare it with an observed working tree. Resolve the
baseline once and show its identity. The working side includes tracked edits/deletions
and non-ignored untracked additions, combining staged and unstaged contents as they exist
on disk. Label that scope; index-only and arbitrary revision-pair modes can follow later.

Coordinate a changed-file overview/list, before/after source with hunks, declaration
context and relevant unchanged source on the selected side. Show added/removed/modified
source without claiming semantic equivalence or transitive impact. Changes outside known
declaration spans remain visible at file level.

Acceptance:

1. Both controlled patch variants can be reviewed against the pinned baseline. The
   walkthrough identifies the seeded regression in the bad variant and does not invent it
   in the correct one, citing before/after source and the relevant contract/test.
2. The change inventory accounts for every in-scope changed Git path, including paths
   outside language analysis. Ordinary UTF-8 changed files remain inspectable without an
   analyser; binary, symlink, mode/submodule-only or explicitly excluded content is described rather
   than omitted. Include a documentation/configuration change in the fixture.
3. Hunk/declaration selection preserves comparison identity and side across all views.
   Removed source is readable from the base, and related unchanged source stays accessible.
4. Baseline source remains pinned across more working-tree updates than the current
   publication-history count. Superseded comparisons, lease expiry and resource refusal
   fail explicitly, preserving the last coherent view.
5. Tests cover additions/deletions, identical declarations, ambiguous matches, unsupported
   analysis, a moved Git ref and edits during capture. Initially represent path renames as
   add/delete; do not infer symbol moves or semantic renames.
6. A non-Git in-memory source provider exercises the same comparison model. Capturing a
   revision under a temporary storage path does not turn every file into a new workspace.

Deliver the frozen patches/oracle, review walkthrough and targeted scale observations.
Publishing comments, approving/merging changes and persistent review management are later
capabilities.

## Architectural boundaries

- Keep acquisition, analysis, investigation state and visual projections separate.
  Git object/ref handling belongs to a native source adapter; the portable comparison
  pairs immutable source snapshots independently of SCM identifiers.
- The current directory adapter derives workspace identity from its canonical root.
  Extract the minimal identity/list/read seam needed for directory and revision inputs;
  temporary Git storage must preserve the logical workspace/path namespace.
- Extend language facts/capabilities through adapters, with provenance, ranges, coverage
  and cache-context invalidation. Start Rust relationships narrowly; do not require
  compiler-backed C/C++ or invent cross-language resolution.
- Keep comparison inventory independent of analyser availability. Freeze compatible
  capture/analysis policy for both sides, and report policy/coverage differences explicitly.
- Investigation/comparison anchors name observations and, for review, a side. Baseline
  ownership needs bounded explicit leases; the three previous publications are not a
  baseline-retention policy. Never read mutable files to satisfy an old snapshot label.
- Native queries supply bounded, renderer-independent projections. Browser and future
  editor hosts consume the same concepts; no editor plugin or production-renderer rewrite
  is required to demonstrate these tasks.

Write the focused behaviour/technical contracts before implementing the relevant
extension, grounded in the walkthrough and inspected code. Specify the new queries,
coverage states and admission/lease limits there; this plan does not preselect a storage
engine, a Git library or a complete graph API.

## Delivery sequence and stopping rules

| Step | Reviewable output |
| --- | --- |
| 1. Baseline learning walkthrough — delivered | [Guided replay](../../experiments/learning-workflow/RESULTS.md), source evidence, recording and observed gaps using current views. |
| 2. Choose the smallest useful extension — next | Specify bounded investigation navigation to address the observed return-context gap; keep a graph deferred. |
| 3. Demonstrate milestone 1 | One complete learning workflow, with any required implementation and meaningful TDD/mutation evidence. |
| 4. Freeze the review fixture/contracts | Good/bad patches, change semantics and source-provider/pinning boundaries. |
| 5. Build and demonstrate milestone 2 | One complete Git-based review workflow plus provider-independent correctness tests. |

Work in small vertical slices. Tests should cover public behaviour, stale/expired anchors,
source identity, language-capability gaps and comparison correctness; use mutation testing
for the new analysis/comparison logic. Keep test assertions independent of private call
sequences. Separate correctness checks from release-build performance measurements.

Use lpc-rs for the complete tasks and focused Linux/PyTorch cases for scale and mixed
coverage. Check the actual new views' payload/DOM bounds and ordinary-edit responsiveness
against the [provisional budgets](../performance/BUDGETS.md). A truncation must expose
hidden counts and a way to refine or page the view. Record first-use delays as task friction.

Further infrastructure initiatives need a demonstrated blocker in an accepted task, a failing
regression or a measured resource violation for that workload, with a bounded fix and
revalidation target. An experiment's unanswered follow-up is not sufficient by itself.
Do not gate these milestones on exhaustive pressure testing, a GPU comparison or all
language relationships. Reopen those questions when the workflows supply the relevant
requirements. Stronger usability, portability and hardware claims remain separate gates.
