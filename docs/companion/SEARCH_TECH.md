# Native question-search trial architecture

Read [the behaviour contract](SEARCH_PRODUCT.md) first. The existing portable search code
is extracted from vizzi-qa into vizzi-search, a safe Rust 2024 workspace crate depending
on vizzi-core and existing workspace dependencies. The QA acquisition/stdio runner stays
in vizzi-qa with compatibility re-exports. The native viewer can then use search without
depending on its own QA adapter or creating a crate cycle.

Two input adapters share one corpus: QA text files retain standalone raw parsing; native
captured Documents reuse their validated source/analysis. Source reads are verified and
bounded, with no inspected project execution or working-file substitution. Document-version
identity controls captured-input reuse and includes content, classification and analysis;
same bytes with different facts must invalidate metadata. Captured inputs do not trigger
another hidden parse when analysis is deferred/unavailable. Serialized QA contexts retain
their interface. Suggested search roles are exposed separately through the portable context.

A native search application uses the current immutable snapshot and one bounded corpus
cache keyed by native snapshot identity. Hold no publication lock during reads/indexing/
queries. Keep native file/version bindings for results; do not equate the search corpus's
internal hash with a native snapshot or content ID. The cache holds bounded copied corpus
data and small bindings, not a persistent Arc to an entire retired snapshot. Reuse unchanged
captured document versions across publications. Verify current snapshot before returning
results; stale work is refused, and browser sequence tokens discard superseded responses.

At most 4,096 UTF-8 documents, 32 MiB corpus text and 2 MiB per document are admitted in
path order. Unsupported/oversize/capacity omissions are explicit counts. Captured storage
errors do not become silent omissions. Search keeps existing window/posting bounds.
One native search operation is admitted at a time on a blocking worker outside the HTTP
event loop. Existing captured analysis is reused, so no second parser pool is introduced.

`POST /api/search` takes native snapshot, question and focused/original selector, denies
unknown fields and caps question text at the investigation's 2,048 UTF-8 bytes. Return
native snapshot, finite role groups, canonical file/version/path/line, literal excerpts
and scope counts. Roles are suggestions, not semantic validation. Responses cap at eight
results and 4,000 excerpt bytes. GET capabilities/status performs no indexing/source IO.

`POST /api/search/evidence` accepts the existing explicit pin fields but checks current
capture before acquiring the evidence. Reuse investigation revision/persistence handling.
The check is immediately before acquisition, not a filesystem/editor-buffer guarantee;
later changes appear as saved-source freshness. Existing saved-item editor handoff rechecks
working content through its verified adapter. No executable or arbitrary source is accepted
from HTTP. Original companion/discovery contracts and default routes remain intact.

The trial URL is opt-in UI state. A small browser module owns query sequencing, original/
focused comparison, grouped result rendering and stale-state actions; the existing preview,
board updates, messages, theme and editor actions are reused. Source/names render as text.
Reuse the existing responsive flex drawer and keep a single input visible. No new renderer,
parser, browser framework or third-party dependency is introduced.

Use TDD through public corpus, native application/router and browser interfaces. Exercise
provided-analysis reuse, changed analysis with unchanged content, body/source bounds,
stale snapshots during requests, cache replacement, source unavailability, no-match and
partial scope, group labels, save-question revision handling and recording-only handoff.
Run serial low-priority mutations on new behavioural modules, preserving unmodified baselines
and reporting exclusions/build failures/timeouts separately. Baseline native/Node/browser
checks finish before release-mode interaction measurements on representative captured source.
