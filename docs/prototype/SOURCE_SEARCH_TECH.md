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

The source search options feed the same pinned query. Literal matching retains
its lexical ranking and can preserve case. Regex matching compiles one Rust
pattern on a fresh search and ranks simple-name and basename matches before
qualified-name and path-only matches. Compilation errors are returned with the
empty match page so the inspector can keep its selection.

The native application retains one ranked search keyed by snapshot, query text, directory,
file types and matching options. Linked views reuse the ranking and matched-file positions
when opening a result or paging the map. The retained query strings and position arrays
have an 8 MiB allowance. Larger queries and evicted rankings are evaluated exactly on the
requested snapshot. The cache stores positions and metadata without holding snapshot handles.
Concurrent searches can complete in any order. Ranking replacement respects request
order, including requests that reused the retained ranking.

The browser debounces search input through the linked-view controller. Scrolling near
the end of the match list fetches a match-only page from the same snapshot through
`/api/matches` and appends the returned rows. The map, inspector and navigation trail stay
in place. Changing the search resets the list, and a mode change also resets the map page.
The navigation trail restores both search options when revisiting a result. The browser
applies a destination after checking its captured identity. The companion uses its existing
preview for direct structural trial sessions.

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

The ranking-reuse comparison ran both optimized hosts on the same 66,594-file Linux
capture with a populated analysis cache. Searching for `init` returned 61,467 matches.
The projected results agreed before and after the change.

| Request | Previous median | Updated median |
| --- | ---: | ---: |
| First results | 217.37 ms | 189.07 ms |
| Second results page | 208.26 ms | 0.39 ms |
| Open a match | 216.75 ms | 3.05 ms |
| Page the filtered map | 214.64 ms | 0.80 ms |
| Return after a different query | 208.87 ms | 189.12 ms |

Each cell represents 12 timed stdio requests on one host, including transport and JSON
decoding. The first request and return after a different query both build a ranking.

The focused ranking-reuse mutation run caught 22 distinct variants after adding the
invalid-pattern file-type regression. One additional variant failed to build.

The search projection mutation run caught 9 variants and 3 failed to build. The
matcher and map paging run caught 14, with 6 build failures and one equivalent
survivor. In `exact_scope`, replacing `<` with `<=` is equivalent because the key
ends in a slash and normalized captured file paths forbid trailing slashes. Both
runs passed their unmodified baseline.
