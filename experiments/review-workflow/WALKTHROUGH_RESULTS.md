# Guided rendered review succeeds on both frozen variants

Both frozen variants now pass the [guided browser runner](run.mjs). It navigates
the actual `/review` controls and records the before/after read policy, unchanged reader
and source-identity contracts, existing regression test, added Markdown, source returns
and refresh/expiry recovery. It uses the normal native HTTP host with one worker.
The [procedure](README.md#run-the-guided-rendered-walkthrough) gives the commands.

## Review evidence

| Frozen variant | Rendered Rust hunks | Guided verdict | Independently checked source rows |
| --- | ---: | --- | ---: |
| A: defective | 3 | Permission errors incorrectly allow suffix fallback | 610 |
| B: control | 2 | Existing lookup rules preserved | 562 |

The defective working side adds `PermissionDenied` alongside `NotFound` in the retry
condition. Its base permits only `NotFound`. The unchanged regression test specifies
one read for `/source-read/denied` with `PermissionDenied`, which contradicts that
broadened fallback. The control retains the base condition. Both guided reviews cite
before/after source and unchanged reader/path/test evidence; no analysed-project test,
build or script is executed.

Inspect the [defective condition](results/browser/variant-a/07-read-policy-horizontal.png),
[control condition](results/browser/variant-b/06-read-policy-horizontal.png), and
[permission-error test case](results/browser/variant-a/12-permission-error-case.png).
Recordings retain the complete routes for [A](results/browser/variant-a/walkthrough.webm)
and [B](results/browser/variant-b/walkthrough.webm). The
[summary and artifact hashes](results/browser/summary.json) bind the results to the
normal release binary, source versions, browser and exact fixture checks.

Both runs retain the added Markdown with an absent base and readable working text.
Both preserve independent vertical/horizontal positions on return, replace the token
on same-content refresh without changing the portable comparison, refuse expired
history while keeping the fresh view, and close without retaining displayed source.
Review SSE is observed; the page makes no status-polling requests. Page/response/cleanup
errors are empty, and both fixtures verify unchanged after exit.

The original learning task also passes again at the same pin. Its selected declaration
and source position return exactly; browser Back stays inside the workspace. Inspect
the [restored learning view](results/browser/learning-regression/07-history-return.png)
or [recording](results/browser/learning-regression/walkthrough.webm). Its 408 checked
visible rows match the baseline archive, and the original checkout remains unchanged.

## Bounds and observed friction

At sampled review checkpoints, each pane has at most **56 source rows** and **26 outline
buttons**. Observed API projections reach 96 source rows, 131 outline entries and 200
file rows; source text stays below the 32-KiB window allowance. The largest observed
projection body is 150,361 bytes. These are observations on this 1,033-path fixture,
not continuous every-frame, heap/RSS or large-repository qualification.

The first comparison becomes ready about **306 ms / 309 ms** after the scripted click
in A/B; host startup takes about **870 ms / 859 ms**. These are single correctness-run
observations. The host has already indexed the live working tree before comparison
creation, so they are not cold review-analysis or tail-latency measurements.

The 1,440-pixel paired layout clips long conditions until the source panes are panned
horizontally. The route now exercises that reading action explicitly. Base/working
horizontal positions restore exactly at **208 / 209 pixels** on Back. Known paths and
lexical outlines are still needed to find related evidence; semantic source links are
unavailable. At 600 pixels the panes stack vertically without horizontal page overflow;
inspect the [narrow screenshot](results/browser/integration/narrow.png).

The operator is reference-aware scripted automation. These results demonstrate a guided
functional task, not unfamiliar-developer acceptance, comprehension improvement or human
task time. Independent-operator evaluation and focused Linux/PyTorch review measurements
remain open.

## Regression and preparation checks

The complete `npm test` pipeline passes all Node/Chromium stages and the review integration
scenario. Native workspace tests (397), formatting and workspace Clippy with all targets/
features and denied warnings pass. Existing mutation evidence for the unchanged
controller/readers and source/bounds audit remains recorded separately; no new product
algorithm is introduced by the added horizontal-reading and screenshot assertions.

The source/bounds audit module was developed from failing tests. Its Node checks verify
exact comparison/file/side/version bindings, UTF-8 byte fragments, CRLF and embedded CR,
absence versus empty text, malformed coordinates, independent response anchors, and
source/outline/file/hunk limits. Wrong labels cannot pass merely because both sides
contain identical text or document versions.

The driver uses response data only to audit rendered state. Visible text is compared
against independently read Git baseline or working bytes. Keyboard outline navigation,
file/hunk controls and browser/toolbar history supply the task actions. The observer
retains bounded windows, while each checkpoint captures its visible source, anchors,
counts and reading positions. Screenshots/video are written only if Chromium starts.

The earlier [preparation evidence](results/walkthrough-preparation.json) records 100 Node
checks, native checks, source fingerprints, blocked launches and fixture reverification
before browser execution became available. It is historical preparation evidence.
The [audit mutation record](results/walkthrough-mutation.json) distinguishes test detections
from survivors. The serial offline runner now accepts an explicit repository-local
test list for this scope; its existing browser-module configuration also passes both
baselines and the activation control without testing any selected mutants.

Reproduce the audit mutation scope with one lower-priority process:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  node_modules/node/bin/node experiments/browser/offline-mutations.mjs \
  experiments/review-workflow/walkthrough-stryker.config.json \
  /tmp/vizzi-walkthrough-audit.json
```

Remaining assertion-message/default-encoding and redundant-guard mutations are documented
individually as equivalent for acceptance/rejection in the audited pipeline. They remain
reported as survivors, with no exclusions and no claim that assertions detected them.
Diagnostic wording is outside that equivalence claim. This does not prove audit correctness.
The final full-module run tested **274 mutations: 221 detected, 53 reviewed survivors,
zero timeouts or runner errors**. Earlier overlapping development runs are recorded
separately. The viewport checks concern rendered rows' DOM text; screenshots still
require inspection for actual glyph visibility, clipping and layout.

## Earlier execution restriction

The earlier attempts stopped at **`host_start`**: the local listener returned
`Operation not permitted`. Both recorded `completed:false`, no browser steps or screenshots,
and successful fixture verification before and after the attempt. The application, browser
layout, keyboard behaviour and rendered good/bad verdicts were not validated by them.
A separate minimal loopback probe fails even earlier, at IPv4 TCP **socket creation**
with errno 1, before `bind` can run. This isolates an environment restriction from
the walkthrough's UI logic.
The earlier Chromium-start restriction is recorded in the
[interface report](INTERFACE_RESULTS.md#initial-browser-attempts-superseded); those
blocked runs did not reach Chromium startup. The later approved runs above do.

Approval-enabled execution outside the sandbox now permits the HTTP host and Chromium.
The successful evidence above supersedes the blocked execution status. Full trace ZIPs
remain outside the committed bundle; their byte sizes and hashes are retained in the
summary. Focused review-scale observations follow this functioning rendered workflow.
