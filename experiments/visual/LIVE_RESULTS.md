# Live working-set validation

For maintainers checking save-driven function bubbles, 2026-11-09.

Working-tree fragments now follow disk saves. Native declaration tracking rebinds each
live view to one coherent observation; source, inline calls and relationship evidence
advance together. Geometry, camera, collapsed state and view identities stay independent.
Unchanged source retains its DOM. Active source selection defers the transaction, and a
changed file remaps the reading position through one nearby unchanged line or explicitly
resets it. **Pin version** holds a view; **Follow live** resumes it. Keep remains separate.

Missing, renamed, cross-file-moved and ambiguous declarations retain earlier source.
Historical arrows keep their old witness and say **previous call** or **removed call**.
Language-server evidence requires another explicit lookup after publication; refresh does
not promote old semantic results. These are conservative tracking limits, not inferred
rename or call-graph analysis. Pinning retains loaded bytes, not a durable source archive:
later paging or reload may encounter native capture expiry. Unsaved editor buffers require
a future editor adapter.

## Correctness checks

Formatting, workspace tests/doctests and Clippy with warnings denied pass: **605 native
tests passed, 3 opt-in provider tests ignored**. The complete browser suite passes,
including the mixed-language live-save, pin/follow, reload and recovery fixture.

Focused native fixtures cover Rust/Python tracking, duplicate/renamed/deleted declarations,
fresh relationship proof, coherent versions, request bounds, long lines and reading-window
boundaries. Model/coordinator checks cover atomic rejection, supersession, cancellation,
retry backoff, selection deferral and mixed pin/live persistence. Browser fixtures exercise
saved edits, unchanged DOM, mixed languages, selection deferral, historical/removed arrows,
reload, missing-function recovery, and retained positions. Reload testing exposed and fixed
the pending layout-save loss on immediate reload. Two retained versions can converge on
one current declaration and still restore as separate views.

No dependency, unsafe-code allowance, inspected repository build or editor launch was
introduced. Automated handoff paths use recording adapters. The native and browser checks
are independent of the actual desktop editor.

After verification, development-build cleanup removed 3.1 GiB of generated artifacts;
the optimised host and browser dependencies remain available. Compressed logs/outcomes
and the two screenshots total under 0.5 MiB.

## Mutation evidence

All interpreted mutation runs have passing unmodified baselines. Native jobs used
`CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10`, `-j 1 --jobserver-tasks 1`;
JavaScript used one lower-priority Stryker worker.

The complete new native refresh module initially had 114 mutations: 74 caught, 35 missed
and 5 unviable. Strengthened boundary/reading tests were followed by 36- and 19-mutation
targeted runs. Taking the latest outcome for each original mutation gives **105 caught,
4 surviving, 5 unviable, no timeouts**. This is a union of archived runs, not a fresh full
114-mutation pass. The four survivors are:

- Removal of `outline_count` from three window requests: reading/header outlines are
  discarded; the visible window's auxiliary outline can grow within its existing bound.
  The visual consumer uses source/call metadata instead. This payload/cost difference is
  not qualified by the behavioural tests.
- Action range subtraction changed to addition: the range only grows, and native action
  ownership already confines results to the function; the original range includes all
  remaining owned sites or reaches the same 96-line cap.

The complete core `resolve`/`check_link` scope finishes with **9 caught, 2 unviable,
no survivors or timeouts**. An initial survivor exposed a missing assertion that a
source-local receiver proof must identify the requested destination; the added test
rejects a different destination even when another receiver target is known.

The complete presentation model and publication coordinator have **1,063 caught,
26 surviving, zero timeouts/errors** in the final 1,089-mutation run. Survivors comprise
overlapping validation/stale-response guards, a unique ticket counter's direction,
equivalent absent defaults/context forwarding, and replacing an already-current link with
the same public relationship. Both stale-success and stale-failure behaviour are tested.
No exclusions hide these outcomes. DOM/controller changes have browser regressions rather
than a complete mutation scope; these results do not establish whole-project coverage.

An accidentally overlapping core follow-up was interrupted immediately; it contributes no
successful verification claim. A zero-match filter attempt and a sandbox-blocked Stryker
logging socket likewise contribute no mutation results. Follow-ups corrected the filters
and used the permitted local socket. Initial and final outcomes remain separate.

## Reproduction and limits

Use `experiments/visual/replay-live.mjs ROOT NEW_OUTPUT` with the optimised binary and a
`tract-plugin-pack` checkout. It copies Minitosis/GsMeter source into a temporary workspace,
opens process → render plus another process view, pins render, applies five sequential
saves, and verifies layout/pinned DOM and later relationship revalidation. It does not edit
the original source or launch an inspected build, editor or language server.

After other verification jobs stopped, five sequential saves measured **105.0 ms median,
109.6 ms maximum** (individual observations: 99.5, 105.0, 109.6, 93.8 and 106.3 ms).
The interval runs from the temporary-file write to current source DOM and enabled current
actions, including capture/indexing, preparation, transport and browser polling; it is not
paint-to-screen latency. The caller's geometry and pinned callee DOM/text remain unchanged,
then Follow live restores a current self-call relationship. No page errors occurred.
The machine has an i7-13700H and 64 GiB RAM, with one parsing worker. The fixture contains
two copied source files, not the full plugin workspace. These observations do not qualify
large-repository indexing. A new observation still constructs the shared structural index
on its first structural operation, even though source parsing is reused incrementally.

[Archived logs, outcomes, source/binary hashes and screenshots](results/live/checksums.json)
keep initial failures and final evidence separate. The initial replay failed before any
timing result because its selector also matched a palette entry; the corrected replay
targets canvas fragments. It contributes no sample to the successful five-save result.

This local known-source replay does not qualify kernel-scale latency, a baseline laptop,
unsaved editor integration or independent usability. See the [usage guide](../../docs/how-to/follow-a-call-trail.md),
[live architecture](../../docs/visual/LIVE_TECH.md) and [interface](../../docs/visual/LIVE_API.md).
