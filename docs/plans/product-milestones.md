# Product milestones: companion workflow checkpoint

The current delivery priority is [one editor-companion learning workflow](../companion/PRODUCT.md).
The prototype below establishes safe browsing/comparison and performance foundations; it
does not establish added value alongside an editor. Further IDE-surface feature work is
paused as a delivery priority. Complete the companion slice and evaluate its contribution
before adding more navigation, outline or diff controls.

The correction after `d647a6e` is to start with automatic source discovery: the earlier
empty question/evidence form assumed the organisation was already known and exposed too
many inputs. The landing page now presents real source areas and names, with one optional
search input and one-click capture. Questions and relationship authoring are disclosed
only when wanted; stored investigations and the editor handoff remain available.

Evaluate discovery with version-bound saved evidence, reader-authored connections,
an on-demand preview and a verified VS Code handoff. Reuse
the existing lpc-rs learning question. Read source in the editor; retain cross-file context
and the explanation in Vizzi. Record a baseline without Vizzi, then the companion attempt,
including duplication, lost context and unsupported questions. Reference-aware automation
checks function, not comprehension or independent acceptance.

The [technical contract](../companion/TECH.md) keeps investigation state separate from
source ownership and from the editor adapter. VS Code is the first adapter, selected by
the maintainer. Persistence/excerpts are bounded and source changes never silently
retarget evidence. The existing review surface remains available; a review investigation
follows the functioning learning slice rather than expanding this slice into a full IDE.

The maintainer also authorised a [CPU repository-QA experiment](../qa/PRODUCT.md).
Its [pilot results](../../experiments/qa/RESULTS.md) and versioned question library measure
external-index retrieval, source support, answer quality, latency, memory and source
changes separately. This does not advance a chat interface or replace source views.
The [compact-evidence follow-up](../../experiments/qa/COMPACT_RESULTS.md) preserves v1,
reviews the seed, freezes fresh formulations and separately tests exact maintainer
questions. Fixed-model latency and source binding improve, while factual coverage does
not; none of the three maintainer workflows is completed. Keep QA outside the viewer.
The [source-complete diagnostic](../../experiments/qa/SOURCE_PACK_RESULTS.md) now supplies
whole relevant definitions and caller/branch context. Partial facts improve, but full
tasks remain incomplete; extra answer allowance does not fix invented caller links or
false refusals. The [focused-question/output-order diagnostic](../../experiments/qa/DECOMPOSITION_RESULTS.md)
now tests the same whole source with guided questions and paired JSON field order.
After the [dispatch oracle correction](../../experiments/qa/DISPATCH_ERRATUM.md), answer-first
raises supported narrow tasks from 3/12 to 5/12 and removes false refusals, but both orders
still fail all three original workflows. The [fixed-source 2B/4B comparison](../../experiments/qa/MODEL_COMPARISON_RESULTS.md)
now improves guided narrow tasks from 5/12 to 12/12, while the 4B model uses 4.76 GiB PSS
and misses the declared 4 GiB target. Neither completes the original workflows under
the rubric. The [physical batch screen](../../experiments/qa/PREFILL_RESULTS.md) now
observes 4.658/4.651/4.645 GiB at sizes 128/32/8 with logical batch fixed at 512. No
profile qualifies, so the cohort stops before full-suite validation. Further allocation
work needs a separate condition. The maintainer's
[weight-space programming proposal](../explanation/model-weight-optimisation.md)
is now tested in a [paired local 0.8B comparison](../../experiments/qa/WEIGHT_EDIT_RESULTS.md).
Both variants complete 1/12 guided tasks and 0/3 broad workflows; false insufficient-evidence
statuses increase from 4/15 to 11/15 with the published edit. A custom edit needs a separate
evidence-use failure hypothesis and development/fresh evaluation split. No planner/chat
interface or production QA acceptance follows from these diagnostics.
The [evidence-use study](../../experiments/qa/EVIDENCE_USE_RESULTS.md) now reproduces errors
in the unchanged 0.8B model when source is permuted or a definition withheld. Its 32
development responses and eight ungenerated real-source reservations are separate.
An ambiguous delivery fixture is flagged; the clearer routing/version/pin cases identify
a behavioural target, not a weight mechanism. Test output-format effects before editing
tensors, and add count-matched controls before qualifying an edit on reserved questions.
The [response-format comparison](../../experiments/qa/DECODING_RESULTS.md) now verifies
equal effective prompts and changes only enforcement. Content remains 14/24 per condition,
with two unique cases helped and two hurt; unconstrained outputs are all prose and fail
the structured interface. Keep the default schema. Specific output-contract and surviving
code-reading failures remain hypotheses, with no weight edit or reserved evaluation yet.
The [generic-JSON follow-up](../../experiments/qa/JSON_OBJECT_RESULTS.md) now parses all
objects but lowers raw content to 8/24 and produces zero host-conforming responses.
Keep the full schema and close simple enforcement relaxation. Remaining model-side
code/evidence failures, count-matched controls and an eventual frozen candidate remain
separate work; reserved real-source questions are still unrun.
The [pin-release token probe](../../experiments/qa/TOKEN_PROBE_RESULTS.md) now observes
small expected-direction source shifts but a persistent No preference and larger layout
effects. It does not locate a weight edit. A cached BF16/Q4 control is the next bounded
model-side check; real-source reservations remain unrun. An explicit mutation-audit
erratum corrects two historical exclusion labels without changing measured QA results.
Retain new maintainer questions for subsequent fresh evaluation. None of these
agent-graded diagnostics establishes product acceptance.

The previous milestone plan and delivery evidence follow as historical context.

# Previous milestones: understand a subsystem, then review a change

Status: the baseline and navigation slice are delivered, with a passing guided learning
workflow. Independent-operator/usability acceptance remains open. The review fixture and
comparison contracts are frozen, and portable inventories/hunks are implemented.
Native acquisition, bounded ownership and shared-engine analysis are delivered.
Comparison queries/transport and linked review views are delivered. Both frozen
variants pass the rendered guided review; the learning workflow passes again too.
Focused Linux/PyTorch review-scale observations are recorded. Review now opens without
starting the live index, with repeated startup measurements. Working refresh preserves
useful reading context, with zero file resets in the repeated scoped cohort.
Independent-operator/usability validation remains open.
This follows the product-alignment checkpoint at commit 4e75d04.

The [human-trial kit](../../experiments/usability/README.md) now supplies neutral
tasks, participant notes, a facilitator protocol and a separate rubric. A maintainer
trial is prepared and awaiting operator observations; preparation and guided replays
do not close unfamiliar-user acceptance.

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
position. The [navigation slice](../../experiments/learning-workflow/NAVIGATION_RESULTS.md)
now restores reading context using bounded session metadata and existing identities.
Workspace declaration lookup is secondary; the originally proposed focused
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
study. The return-context criterion now has regression coverage and a repeated guided
demonstration; independent-operator acceptance is not inferred from that recording.

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
| 2. Choose the smallest useful extension — delivered | [Navigation contract](../prototype/NAVIGATION.md) and implementation address the observed return-context gap; a graph stays deferred. |
| 3. Demonstrate milestone 1 — guided workflow delivered | [Repeated learning task](../../experiments/learning-workflow/NAVIGATION_RESULTS.md) with TDD/mutation evidence. Independent-operator validation remains open. |
| 4. Freeze the review fixture/contracts — delivered | [Verified good/bad fixture](../../experiments/review-workflow/RESULTS.md), [behaviour](../prototype/COMPARISON_PRODUCT.md) and [technical contract](../prototype/COMPARISON_TECH.md) specify change semantics, source providers and pair ownership. |
| 5. Build and demonstrate milestone 2 — guided workflow and focused scale observations delivered | Both frozen variants pass the rendered review, with provider-independent correctness tests and bounded Linux/PyTorch measurements. |

The implementation order for step 5 is:

1. Portable comparison of supplied immutable observations, with an in-memory provider,
   independently specified inventory/diff expectations, TDD and focused mutation tests.
2. Minimal native identity/list/read seam and Git base/working acquisition, using shared
   worker/admission budgets and bounded pair/source ownership.
3. Token/side-bound application queries and transport, with explicit refresh, status,
   capacity/error states and bounded source/hunk projections.
4. Linked review views and keyboard/history behaviour, then the two-variant walkthrough
   and focused task-scale observations. Retain the learning workflow as a regression.

Each slice delivers exercised public behaviour; it does not require building the later
layers first. Fixture validation is preparation evidence, not completed review acceptance.

Step 5 has begun: [portable inventory comparison](../prototype/COMPARISON_TECH.md#delivered-portable-inventory-interface)
now classifies supplied observations and binds source to an explicit side, with non-Git
behavioural tests and [validation evidence](../../experiments/review-workflow/INVENTORY_RESULTS.md).
The [bounded textual diff](../prototype/TEXT_DIFF.md) now computes raw-line hunks on
those same captured documents, checked against the frozen variants. The
[native source seam and source-only Git acquisition](../prototype/REVIEW_ACQUISITION.md)
now supply those bytes directly. [Pair/source admission](../prototype/REVIEW_OWNERSHIP.md)
now protects the active pair, private capture and held retired queries. The
[analysis bridge](../prototype/REVIEW_ANALYSIS.md) now applies the shared engine under
one aggregate worker budget, preserving admitted source owners and the baseline plan.
Bounded, token/side/version-bound [application queries and transport](../prototype/COMPARISON_API.md)
now expose this model. The initial [linked review interface](../prototype/REVIEW_INTERFACE.md)
has [controller/native evidence](../../experiments/review-workflow/INTERFACE_RESULTS.md).
The [rendered review walkthrough](../../experiments/review-workflow/WALKTHROUGH_RESULTS.md)
now passes for both frozen variants, with source/bounds audits, screenshots, recordings
and a repeated learning regression. The
[focused Linux/PyTorch measurements](../../experiments/review-workflow/SCALE_RESULTS.md)
now record actual review interaction, explicit refresh, bounds and memory observations.
The [startup contract](../prototype/STARTUP_PRODUCT.md) now keeps live indexing dormant
until exploration activates it. The [repeated startup cohort](../../experiments/review-workflow/STARTUP_RESULTS.md)
shows URL availability around 4–5 ms and the first scoped review around 0.8–1.0 seconds
from launch, while preserving the learning/review workflows. The
[refresh navigation contract](../prototype/REVIEW_REFRESH.md) and
[repeated context cohort](../../experiments/review-workflow/REFRESH_RESULTS.md) now
preserve file/side/filter, same-version pane positions and surviving exact declarations.
All 30 working refreshes retain the edited file. The [first maintainer feedback response](../../experiments/usability/FEEDBACK_RESULTS.md)
now adds readable syntax/kind colours, overview navigation, resizable sections, explicit
commit pairs and visible input errors. Unfamiliar-operator validation remains
the separate acceptance step; further views should follow demonstrated learning/review
gaps rather than an infrastructure experiment's open questions.

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

The [second maintainer orientation response](../../experiments/usability/ORIENTATION_RESULTS.md)
adds actual directory containment and personal ordering/collapse, dismissible messages,
system/explicit appearance and stronger syntax colours. It also corrects the trial
history limitation; independent comprehension acceptance remains open.
