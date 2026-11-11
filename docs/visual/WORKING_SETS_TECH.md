# Working-set architecture

For contributors implementing [the behaviour contract](WORKING_SETS_PRODUCT.md).

A TaskWorkspace extends the existing source workspace with group and result-query
descriptors. Source views and source relationships keep their existing representations.
Reference rows, previews and active requests belong to a dedicated renderer and expire
with their query observation. Generic canvas geometry includes source and result items.
Group bounds derive from member geometry. Group operations validate the complete move
before applying it. Collapsing retains member positions for later expansion.

A task-shelf module validates complete workspace documents and persists the current task
and named tasks in one bounded storage bundle. Legacy source-workspace JSON is the migration
input. It delegates source validation to the existing workspace interface. Browser source
bytes and native paths do not enter the task bundle.

Native reference queries resolve the selected native declaration identity before asking
the configured language provider. Rust uses standard LSP references over the captured
private mirror. Positions must map to current captured documents and admitted declarations.
Caller filtering additionally requires an exact captured call identifier at that position.
The response distinguishes semantic results from name candidates and reports locations
outside admitted source. Query results have a finite cache and observation-bound paging.
Opening a semantic caller uses its native call witness and target. Source changes reject
late results. Other providers can add reference capability through the definition-provider
interface without changing the browser data model.

The native query cache holds four result sets with at most 256 rows each, paged eight
at a time. The Rust LSP adapter accepts at most 1,024 returned locations. References map
to captured repository documents or dependency files already admitted by the provider;
this feature does not scan dependency trees. Unknown locations are counted separately.
Without semantic reference support, the fallback offers written calls with the same
terminal name, rather than claiming to find all non-call uses. These are candidates,
including when only one candidate remains.

Task layouts admit 24 source bubbles, four result stacks and eight groups. The shelf
holds at most 12 tasks in a single 3 MiB browser-storage bundle; a layout is limited to
512 KiB. It is local to the browser and origin, without cross-device synchronisation.
Persisted result descriptors are re-queried; they do not retain result source bytes.

Task groups, language modules and source origins retain independent meanings. Task shelf
state and layout updates never change analysis certainty or source contents. A task switch
aborts source/result work before mounting another workspace. Existing layout keys remain
available as migration data until the new bundle is written successfully.

[Validation evidence](../../experiments/visual/WORKING_SETS_RESULTS.md) records the
browser/native checks and mutation-test limitations.
