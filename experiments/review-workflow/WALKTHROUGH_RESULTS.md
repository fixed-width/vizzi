# Rendered review walkthrough: preparation and blocked execution

The [guided browser runner](run.mjs) is prepared for both frozen variants. It navigates
the actual `/review` controls and records the before/after read policy, unchanged reader
and source-identity contracts, existing regression test, added Markdown, source returns
and refresh/expiry recovery. It uses the normal native HTTP host with one worker.
The [procedure](README.md#run-the-guided-rendered-walkthrough) gives the commands.

## Verified preparation

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

The [preparation evidence](results/walkthrough-preparation.json) records passing Node
checks, native checks, source fingerprints, both launch failures and fixture reverification.
The checks pass **100 Node tests** (including 12 audit tests), **397 native workspace
tests**, formatting, JavaScript syntax checks and workspace Clippy with all targets/
features and denied warnings. Those counts exclude Chromium acceptance.
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

## Execution still blocked

Both attempts in this session stopped at **`host_start`**: the local listener returned
`Operation not permitted`. Both recorded `completed:false`, no browser steps or screenshots,
and successful fixture verification before and after the attempt. The application, browser
layout, keyboard behaviour and rendered good/bad verdicts have not been validated by them.
A separate minimal loopback probe fails even earlier, at IPv4 TCP **socket creation**
with errno 1, before `bind` can run. This isolates an environment restriction from
the walkthrough's UI logic.
The earlier Chromium-start restriction is recorded in the
[interface report](INTERFACE_RESULTS.md#browser-acceptance-still-open); this run does not
reach Chromium startup and makes no new claim about that stage.

The next acceptance step requires an execution environment that permits the local
listener and Chromium. Run both variants, inspect the recordings/screenshots, address
any interaction failures, then repeat the learning workflow as a regression. Task-scale
payload/DOM/interaction observations follow that functioning rendered workflow. No new
product feature or performance qualification is substituted for the missing evidence.
