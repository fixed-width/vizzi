# Coordinated source search

The [behaviour contract](SOURCE_SEARCH_PRODUCT.md) covers the Source browser and
the Saved locations entry point.

## Query and presentation

The pinned linked view query projects the map, a bounded match page and the inspector
from one observation. Match scope, map type filtering and declaration admission use
the captured documents. Like companion discovery, source search traverses all their
lexical facts, including those outside the call-oriented structural projection.
Source identities use the selected document version and declaration entity.
Diagnostic callers use the path-only view filter by default.

The browser debounces search input through the linked-view controller. Results and
map pages have independent offsets. The match list uses the existing inspector and
navigation trail. A result is only applied when the proposed destination agrees with
its captured identity. The companion uses its existing preview for direct structural
trial sessions.

## Validation

Exercise exact and qualified matching, duplicate declarations, path-only hits,
type and directory scoping, result paging and map membership with deterministic
native fixtures. Exercise the coordinated map, results, source and history through
Chromium, and check the saved cards and the direct structural trial route. Run the
workspace baseline checks and focused low-concurrency mutation testing.

Formatting, Clippy, 923 workspace tests and the 92-file browser suite passed. Four
queries against the local lpc-rs capture returned every legacy destination, and the
browser suite exercised map paging in a fixture with more than 2,000 files.

Browser regressions cover typing during a match request, clearing a search revisited
through Back, revealing a deeply nested tile and reopening the current location. All
four failed on the original implementation and passed after the fixes.

In a separate warm release-host run on lpc-rs, 30 stdio requests each for
`read_file`, `reader` and `src/compiler/mod.rs` had medians of 18–42 ms and
95th percentiles of 21–44 ms. Timing includes stdio transport and JSON decoding.

The search projection mutation run caught 9 variants and 3 failed to build. The
matcher and map paging run caught 14, with 6 build failures and one equivalent
survivor. In `exact_scope`, replacing `<` with `<=` is equivalent because the key
ends in a slash and normalized captured file paths forbid trailing slashes. Both
runs passed their unmodified baseline.
