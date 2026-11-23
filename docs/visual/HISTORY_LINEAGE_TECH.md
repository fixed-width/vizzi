# Adjacent function correspondence

[Product contract](HISTORY_LINEAGE_PRODUCT.md). Function history already has immutable
Git captures, declaration identity and exact name spans from the registered syntax adapters.
Keep lineage in the history owner and reuse captured source for comparison and call queries.

- Anchor the query to the oldest accepted version of the visible timeline. Use its
  sole Git parent and bound the changed-path inventory. Read at most 16 paths of the
  configured language, 2 MiB per file and 8 MiB of source across both commits. Require
  complete syntax and readable blobs for files used in the proof. Compare exact bytes
  on either side of the parser's name span within the full declaration. For a C-family
  prototype, the complete span includes the return type when the parser can attribute
  the enclosing declaration to one declarator. Compare the scope encoded in the native
  declaration key, including scopes that do not have outline rows. Both sides must have
  one matching function, with distinct name or path identities and no surviving or
  pre-existing declaration with the relevant tracking key. Git path changes select inputs.
- Preview keeps the accepted history unchanged. Acceptance repeats the checks against
  the exact caller pins and selected candidate pins, then appends a version at the parent
  commit. Store the declaration binding with each version so historical paging and
  comparison reacquire the correct file. Keep the 64-file-revision bound across paths
  and at most four accepted transitions per session. Once the reader accepts, the
  history owner can finish the transition even if a newer browser intent supersedes
  the response. The displayed version changes only when the response still applies.
- Carry ordered native correspondence metadata in history frames and comparison sides.
  Validate saved correspondence against the original and displayed declaration identities.
  The browser uses the existing preparation and source-intent checks before changing
  the bubble. Live refresh uses the original binding when returning from a timeline
  with an accepted transition. When a caller moves, links retain the earlier caller
  file and path so the saved task can validate the original call witness. Caller results
  match either accepted identity when the comparison crosses the transition, and reveal
  all sides of that comparison that remain open.

## Validation

- Deterministic fixtures cover simple rename and move, independent ordering, comparison,
  captured calls, duplicate declarations, continuing copies, gaps, changed scopes,
  wrong pins and unavailable source.
- Chromium checks explicit acceptance, cancellation, reload, keyboard access, geometry,
  adjacent comparison and Return to live. Run the native and browser regression suites
  and focused serial mutation checks.
- Replay lpc-rs's `Register::as_register` → `Register::as_local` at `3f145cef2793` and
  the Vizzi Rust tutorial file move at `c61a891`. Record selected declarations and bounds
  separately from fixture correctness.

The native history suite passed all 43 tests. The full workspace tests, Clippy and
default Chromium suite passed with their opt-in checks excluded. The focused serial
mutation run caught all 11 selected correspondence mutations after a passing baseline.

### Reference replays

The optimized native host found and accepted both transitions, then returned to the
exact newer file and commit through Next change.

| Repository | Newer commit | Parent | Earlier declaration |
| --- | --- | --- | --- |
| lpc-rs | `3f145cef2793` | `a75c86de0334` | `Register::as_register` in `lpc-rs-core/src/register.rs` |
| Vizzi | `c61a8912aea3` | `56a63871b360` | `Greeter::greet` in `docs/tutorials/first-call-trail/src/lib.rs` |

The preview took 339 ms for lpc-rs and 14 ms for Vizzi. Acceptance took 169 ms and
175 ms respectively. These are single local HTTP observations after indexing and exact
commit selection, with two workers and a fresh cache for each repository.
