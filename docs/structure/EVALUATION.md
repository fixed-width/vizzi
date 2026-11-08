# Structural experiment evaluation protocol

For contributors evaluating the explicit inventory/call-site prototype. This protocol
separates a useful navigation operation from natural-language interpretation and human
comprehension. Neither is established by a passing browser replay.

## Freeze before unseen source

Finish development using the independent Rust/Python fixtures in the core/native/browser
tests. Record the candidate source hashes, release-binary hash and completed checks before
exporting or searching the reserved repository. Keep its verbatim questions and source
revision in the reserved cohort. The Tract cases remain observed evaluation cases, not
new development fixtures or fresh results.

For each reserved question, an evaluator may choose an explicit operation and a small set
of literal search seeds from the question **before source inspection**. Write those choices
down before running them. This is human/agent task interpretation, not an automatically
qualified natural-language router. Do not introduce repository-specific names learned
from source or expand ranking rules during the run.

## Capture first observations

1. Capture the unchanged original/focused question-search responses for control, before
   source-aware navigation. Preserve questions, snapshot, scopes, bodies and order.
2. Capture source-area/name results and inventory for the preselected literal seeds. Follow
   candidate calls only from a deliberately selected declaration. Keep same-name ambiguity
   and unresolved steps; never invent an edge to complete the task.
3. Run ordinary `rg` name/text search with the same literal seeds and source scope. Record
   commands/results and distinguish an optional broader documentation search from the
   indexed-source comparison. Do not handicap the baseline by making it interpret whole
   prose while giving structural operations hand-picked symbols.
4. Preserve outputs before auditing expected locations. Query/UI errors and missing
   language/analysis coverage are outcomes, not permission to change the candidate.

## Audit usefulness separately

After first observations, inspect source to establish reference locations and limitations.
Record useful starting points, missing declarations/use sites, misleading candidate links,
required queries/selections, source inspections and manual fallback. A declaration reached
by file orientation is different from a returned runtime/registration anchor. A count of
written same-name calls is not proof of actual uses.

Keep a small source-informed replay for observation/version binding, trail/back, previews,
location keeps and recording-only editor handoffs. Report whether retained source contains
the supporting call spelling. This does not measure independent navigation time or success.
Compare automated operation counts only as descriptive proxies: the evaluator has already
seen source, previews/search result bodies carry different amounts of information, and
correctness may still require reading a candidate. Do not claim reduced human effort from
fewer scripted clicks alone.

Independent maintainer use is the acceptance check: does the inventory/call-site view make
finding and retaining relevant source clearly easier than the editor/text-search baseline?
If no clear advantage emerges, leave natural-language search unqualified and stop expanding
it. A later model trial needs a separate frozen intent/selection contract, development set
and another unseen evaluation; it must keep source identities and unresolved links intact.

## Costs and cleanup

Complete tests/mutations before optimized-build timing. Use one indexing worker and nice 10;
report cold preparation separately from repeated metadata query/interaction costs. Timings
on a developer machine do not qualify hardware frame times or large-repository memory.
Use immutable local exports and external caches, execute no inspected-project tooling, stop
owned hosts/browsers, and use recording editor adapters during automation.
