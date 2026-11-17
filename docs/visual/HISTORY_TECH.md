# Function history architecture

Read [the behaviour contract](HISTORY_PRODUCT.md). Inspected baseline:
`7a41c8ac166cf26cf7048afb8673cbfa54b70978`.

## Context

`web/visual-model.js` already separates stable views from immutable source bindings.
`review_source/git.rs` supplies bounded, read-only Git object acquisition, while
`engine/review.rs` analyses immutable input with the configured engine and worker budget.
Structural call routes remain current-observation-only. The review store owns a single
active comparison and must not be used as a bubble-history registry.

## Changes

- Add a native function-history owner, separate from live publication and review. Validate
  initial snapshot/file/version/tracking against a unique native function fact. Retain its
  lexical key and analysis language, never a browser-supplied path or declaration name.
- Extend Git acquisition with literal single-path revision capture and bounded first-parent
  commit discovery pinned to resolved HEAD. Read raw regular blobs only, using existing
  command/environment and object-size protections.
- Lazily scan up to 64 file revisions for a different exact declaration-byte digest.
  Analyse whole affected files through the host engine, then match native unique lexical
  keys. Missing/ambiguous declarations are boundaries. Revisit accepted commits through
  immutable object reads; keep just the selected analysed file per session.
  Scan consecutive equal declaration bodies to their earliest known commit, keeping the
  next distinct candidate for the following step. Commit cursor changes only on success;
  temporary analysis refusal during lookahead remains retryable.
- Bound sessions to 24, retained source bytes to 32 MiB and each source file to 2 MiB.
  Oldest sessions expire when admitting another would exceed limits. Metadata and parser
  overhead remain outside the logical source allowance. Serialise history work with a
  nonblocking owner lock and the existing bounded HTTP worker allowance.
- POST `/api/function-history` starts a session. POST `/api/function-history/step` selects
  an accepted zero-based index or discovers its next older version. GET
  `/api/function-history/window` pages an accepted immutable snapshot/file/version. The
  owner can reacquire an earlier accepted capture if a discarded preparation changed its
  selected cache, so cancellation cannot invalidate a retained reading binding.
  Start/step replies include token, pinned tip, commit, index, symbol, snapshot, first
  source window and declaration-origin row. Errors preserve displayed state.
  Start accepts `snapshot`, `file`, `version` and `tracking`; step accepts `token` and
  `index`. Window queries bind `token`, `snapshot`, `file` and `version` alongside the
  existing WindowQuery fields. Invalid inputs return 400, stale or expired bindings 409,
  and acquisition/analysis failures, busy states and history boundaries 503. The shared
  companion transport bounds request bodies to 16 KiB and work to two operations.
  History windows use the shared abortable source-read retry policy for 429/503 responses:
  six attempts with 50–800 ms exponential waits. The same snapshot/file/version/token is
  retained on every attempt. Expired 409 responses and history start/step operations are
  not retried. This lets concurrent declaration-header and section reads recover from the
  history owner's nonblocking admission lock, including when reacquiring an earlier capture.
- Extend presentation state with optional history metadata and the original live binding.
  History is pinned independently of the global live snapshot. Switch only after the
  prepared source validates; invalidate outgoing/incoming arrows, old call intent and
  source requests. Preserve geometry. Return resumes the unique declaration tracking
  binding while retaining the displayed historical capture until live refresh resolves
  it. Missing declarations retain that exact history binding and paging route; only a
  successful native refresh removes history metadata.
- Keep source reading/paging on SourcePane. Historical source uses the history window
  route and never requests live call actions. Persist metadata rather than source bytes.

## Validation

Use deterministic Git repositories to exercise skipped unrelated edits, saved working
changes, position movement, forward revisits, frozen HEAD, lexical ambiguity, missing
functions, literal paths, shallow history, expiry and exact window bindings (1–7, 9).
Use presentation tests for stable layout, persistence, invalid inputs, link freshness and
return-to-live (2–4, 8–10). A real browser test covers history navigation, independent
bubbles, paging, reload and current-source actions. Observe failing focused tests first.
Run formatting, workspace tests/doctests, Clippy and browser checks. Use serial nice-10
cargo-mutants after a passing baseline; record survivors and incomplete checks separately.
Measure optimized history operations separately from correctness, without asserting a
latency budget from fixture results.

## Observed validation, 2026-11-16

The later [complete browser qualification](../development/function-history-validation.md)
records final-PR and merge-commit CI separately from the focused checks below.

Linux verification passed formatting, workspace tests/doctests (758 passing tests,
including 14 native history tests), Clippy with warnings denied, 263 browser model tests,
and eight packaged Chromium workflows: function history, live workspace, visual workspace,
working sets, dependency workspace, workspace controls, function margins and source reflow.
The history workflow includes cancelled prepared replies, missing-function retention and
recovery across reload, and usable controls at the minimum bubble height.

Scoped cargo-mutants checks covered history selection, matching, accepted-version paging,
initial windows and the Git history/file adapters. Across 54 unique mutations, 50 were
caught by failing tests, two timed out and two could not compile. The timeouts replaced
cursor increments with multiplication by one and stopped traversal progress. The invalid
builds generated Default values for Capture/Hash and GitRevision, which have no Default
implementation. Neither category is counted as caught by tests. Three initial survivors
exposed missing assertions about response bounds and original line endings; a new interface
test caught all three on rerun, alongside a fourth previously caught window mutation.
No equivalent mutations were excluded and no survivors remain in this selected set.

Runs used nice 10, one mutant job, one jobserver task, CARGO_BUILD_JOBS=1 and
RUST_TEST_THREADS=1, with passing unmodified baselines. `-C=--test=function_history` bounds
both mutation build and test phases. A scoped run resumed with `--iterate`; results above
combine unique mutations across its passes and the window regression rerun. An earlier
exploratory full-module run was interrupted when commit attribution was refined; its
partial results are not included. This is scoped evidence, not full-repository mutation
coverage. The complete browser suite and large-reference-repository history qualification
were not run.

A separate optimized, one-worker observation on Vizzi itself (269 captured source files)
measured Structure::new history at 213.75 ms for the first earlier version and 10.73 ms
for a revisit. This is one local observation, excluding initial indexing, and does not
establish a latency budget for Linux or PyTorch.

### Busy-read restoration regression

Review reproduced a later-section reload failure: concurrent header and section reads
could receive 503 from the history owner's nonblocking lock, while the shared retry
allowlist omitted the history window endpoint. The endpoint now uses the existing bounded
source-read retries. Before the fix, both focused retry tests and the local-server browser
regression failed; afterward they passed. The browser test moves the server's selected
capture away from the retained version, injects one busy response for each parallel read,
and checks recovery with unchanged pins, revision, layout and reading section.

Follow-up verification passed workspace tests/doctests, Clippy, formatting, all 267 browser
model tests and the packaged function-history Chromium workflow. The retry helper's ten
tests also cover retry limits, cancellation, expiry, navigation requests and cleanup of
abort listeners/timers. A serial nice-10 Stryker instrumenter run checked all 45 mutations
of read-retry.js after passing unmodified and instrumented baselines: 35 were caught by
failing tests and ten hit the three-second process timeout. Timeout variants removed
promise completion or retry termination; they are reported separately, not counted as
caught. No survivors, runner/build failures or equivalent exclusions remained. This
follow-up changed JavaScript behavior only; native mutation checks above were not repeated.
