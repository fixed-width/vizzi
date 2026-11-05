# Pane information and declaration structure refinement

This record covers maintainer corrections after `7de79b7`. Dismissal now applies only to
full-width ephemeral banners outside the panes. Capture labels, evidence, pane help,
inline readiness, source availability and declaration empty states are persistent text.

Group headings use measured glyph ascent/descent and centring within the allocated
header band. If the whole glyph box plus vertical margins cannot fit, the label is
suppressed at that zoom. The directory controls continue to expose names. Browser tests
audit the actual canvas glyph boxes across zoom interactions; pure tests cover additional
scales and font metrics.

Declaration names remain aligned rather than accumulating indentation. Each current
window entry adds reported depth and a compact nearest-enclosing-scope label; top-level
entries have a stronger name weight and a File outline context. Third lines retain the
48-pixel virtual row height. Built-in Rust, Python, C and C++ adapters record parent links
from their existing syntax traversal, using the same named scopes as their identities.
The model validates earlier-parent ordering and strict source containment per analysis
context. Labels are at most 24 UTF-8 bytes; the depth array is built once at admission.
No source parsing or ancestor-chain traversal happens during viewport requests. Legacy
adapters without links remain flat; the hierarchy describes emitted lexical scopes,
not semantic ownership or every anonymous source block.

Available workspace/review captures keep their readiness text while reanalysis is pending.
Unchanged text is not rewritten, avoiding live-region churn. Initial capture and real
failures remain explicit. Existing busy state and disabled controls describe pending work.
The browser regression injects valid pending/ready notifications through the public SSE
consumer and verifies stable text and zero readiness-text mutations.

## Verification

Formatting, 431 workspace tests including doctests and all-target/all-feature Clippy
passed. The final browser runner has 222 passing Node/Chromium cases plus the
integrated startup/review/refresh/refinement flows.
Native tests cover all built-in scope adapters, sibling/implementation/nested-function
structure, deep paging, multiple contexts, malformed parents, one-shared-boundary cases,
Unicode label limits and adversarial JSON. The existing 512-KiB document-window bound
still passes with escaped hierarchy labels, source, names and syntax spans. Browser checks
cover external-only dismissal, depth-30 aligned names, persistent pane information,
readiness stability, zoomed headings and narrow layouts. Existing startup, learning
navigation, review and refresh regressions remain checks in the full runner.

The [compact validation record](results/refinement-validation.json) retains hashes and
per-mutant outcomes. Native mutation scopes cover the complete changed admission, entry
projection and syntax-traversal functions. A focused follow-up added valid/invalid shared
span boundaries and the existing colouring-validation tests to the cohort. The remaining
native variants include a redundant self-parent guard (strict containment already rejects
it), redundant matching of the current scope's sole emitted declaration, and cooperative
cancellation-check timing. They are not counted as caught. Unviable builds and timeout
variants are separate. JavaScript mutation checks cover complete heading-placement,
readiness-label and window-validation modules; remaining diagnostic/defensive guard and
layout-threshold variants are retained. No exclusions were used; every cohort established
a passing baseline, with serial lower-priority jobs, compiler tasks and test threads.
Coverage and mutation results do not prove correctness.

Optimized interaction observations are collected separately after tests and mutations,
using the 2,000-directory fixture and pinned lpc-rs source. The record contains their
source/binary fingerprints, CPU/rAF distributions and resource bounds. These are limited
development-host observations, not a laptop envelope or unfamiliar-user acceptance.

The separate optimized observation retained an unchanged source snapshot in both the
2,000-directory fixture and pinned lpc-rs. Observed rAF p99 was 16.8 ms in each (92/93
pan samples), with drawing CPU p99 2.4/2.8 ms and no observed pan long tasks. Directory
controls stayed at 11 rendered rows; source/outline bounds held. These small samples
measure scheduling/CPU work, not physical presentation or timing-tail guarantees.
