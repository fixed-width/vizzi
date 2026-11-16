# Captured-file call exploration

Read [the behaviour contract](CAPTURED_CALLS_PRODUCT.md). Extend the existing history and
function-comparison implementation; current-only structural and semantic routes keep
their existing authority checks.

## Changes

- Add a captured-file query adapter to FunctionHistory. Requests contain an owner
  (`history` token/index or `comparison` token/side), exact snapshot/file/version and
  native caller symbol, plus actions, calls, definition or destination-open operation.
  Verify accepted owner and source pins before native Structure queries. Reuse the
  single-file capture and existing source-local/name candidate evidence, never a live
  structural index or semantic provider.
- POST `/api/captured-functions` is an idempotent read projection and uses the existing
  bounded source-read retry policy. Preserve 16-KiB request and native work admission
  limits. Calls page at eight, actions at 256 within 96 lines; the browser retains its
  4,096-action section bound. Open validates both the call's containing declaration and
  membership of the selected target in its actual native candidate set before reading.
- History lookup validates the accepted ordinal as well as exact source pins; an old
  accepted capture can be reacquired from immutable Git objects. Comparison lookup
  borrows its owned side under the existing quota. Each operation holds its owner while
  projecting metadata/source and returns bounded owned output. No new source registry,
  whole-project snapshot retention, checkout, fetch or language-server session is added.
- Persist a per-view captured owner/label for newly followed declarations. Keep it distinct
  from function-history tracking: the caller's history session does not track the callee.
  Source paging reuses the original history/comparison window route. Native captured call
  queries accept any exact admitted declaration in that file, enabling recursion/chains.
- Separate call-navigation availability from current-source usability. Frozen views can
  follow captured calls while Editor/Save/references remain gated by current source.
  Prepare actions alongside source; verify source binding and navigation intent again
  before adopting a destination. Existing editor-delivery cancellation also aborts this
  preparation and invalidates its navigation ticket.
- Add a captured-link state with original evidence and source witness. Retain it through
  live refresh only when both endpoints still share the exact owner/source context.
  Changing either end degrades it to an unverified historical relationship. Validate
  persistence and distinguish revision context in deduplication; never create a call
  relationship solely from a bubble's spatial placement.
  Return to live degrades attached captured links immediately, so saving/reloading while
  live resolution is pending retains a valid layout even if refresh cannot complete.
  It also cancels older editor intent and dismisses captured destination choices before
  waiting for live resolution; a failed refresh cannot leave an obsolete chooser active.

## Validation

Use native fixtures with diverged working-tree/captured helpers, ambiguous method names,
Rust self bindings, unresolved cross-file calls, recursive chains, wrong caller/call/target
pins, expiry and comparison sides. Test public model interfaces for atomic bounds,
capture-aware reuse, groups, persisted links and live transitions. Exercise real Chromium
inline and Calls navigation, later sections, comparison captures, reload, current-provider
isolation, selection and delayed replies superseded by editor navigation. Run the repository
baseline and scoped serial nice-10 mutation checks after a passing unmodified baseline;
report survivors, invalid builds, exclusions and incomplete checks separately. Keep
optimized timing observations separate from correctness and state their workload limits.

## Observed validation, 2026-11-16

Final Linux checks passed formatting, workspace tests/doctests (774 passed, with 15
opt-in integration checks ignored) and Clippy with warnings denied. The packaged release
was rebuilt and captured calls, history, comparisons and comparison/editor cancellation
were rechecked in Chromium after resuming the interrupted work.

Native Git fixtures cover old helper bodies beside diverged current source, recursive
chains, Rust self bindings, ambiguous names, unresolved cross-file calls, partial analysis,
exact caller/source/owner checks and comparison sides. Destination frames also check
declaration-only windows, literal line endings, bounded outlines and the complete origin
row of a heavily indented function. The history/comparison integration suite passes all
26 tests.

Packaged Chromium checks passed captured inline/Calls navigation, recursion and capture-aware
reuse, ambiguity, later sections, provider isolation, live updates, reload, comparison sides
and newer-editor cancellation. Existing history, comparison, editor delivery, live workspace,
task groups, dependencies, controls, function margins and reflow workflows also passed.
The final captured-call workflow also first reproduced and then checked dismissal of
captured choices when Return to live runs while the live-refresh transport is unavailable.
The model/delivery suite passes 309 checks, including a regression that first reproduced
invalid saved links when Return to live preceded a successful refresh. Removing the fix
was caught by a failing assertion in a separate, baseline-checked browser-model mutation.

Scoped cargo-mutants runs covered the complete captured-file adapter, rather than only
changed lines. Of 34 unique mutations, 29 were caught by failing tests and one generated
replacement could not compile because Invocation does not implement Default. There were
no timeouts. Eight initial survivors exposed missing destination-window assertions; their
behaviour-changing variants were caught after strengthening the fixtures and rerunning.
Four remaining survivors are narrowly classified as equivalent:

- Replacing `||` with `&&` in the caller's file/version check: owner admission has already
  validated both pins against the only document; exact symbol resolution uses that same file.
- The two `||` to `&&` replacements in the history capture's snapshot/file/version check:
  the accepted snapshot is checked before reacquisition; its identity includes the sole
  document version, which includes its file identity, contents and analysis. The inner
  declaration check also rejects mismatched file/version pins. These replacements cannot
  admit a different observation through the public interface.
- Removing `outline_count` only from the origin-row WindowQuery: the response retains its
  first source line and discards outline metadata. This does not exclude the destination
  window's outline bound or either query's source-line bounds.

These equivalents are reported separately from failing-test catches; no broader source-pin
or window mutations are excluded. Runs used nice 10, one mutant job, one jobserver task,
CARGO_BUILD_JOBS=1 and RUST_TEST_THREADS=1, with passing unmodified baselines. This is evidence
for the native adapter and the targeted model transition, not complete browser-controller
mutation coverage. No new large-repository latency qualification or optimized captured-call
timing was collected.
