# Explore inventories and written calls beside your editor

For a developer trying the structural prototype on a local repository. It searches
captured source areas/names and exposes raw call candidates. It does not interpret a
natural-language task, resolve imports/types, or reconstruct runtime/data flow.

Launch the native viewer with external state and one indexing worker:

```sh
CARGO_BUILD_JOBS=1 cargo run --release -p vizzi -- \
  --root /path/to/repository --cache /tmp/vizzi-structure-analysis --workers 1
```

Open the printed address with `/investigate?trial=structure` appended, or choose
**More → Try structural exploration** from the companion. The default overview and
question-search trial remain available.

Enter a literal source-area or declaration name. Choose a returned directory/file to
see its inventory. A name filter stays applied when its file opens; **Area inventory**
shows all declarations in that area. Use **Next results** for further pages.

Each declaration has its exact captured source location and counts of same-name call
sites/declarations. These are candidate matches. A count of zero does not prove that a
declaration is unused; callers may be indirect, generated, unindexed or unsupported.
The coverage line reports partial/unavailable syntax and metadata allowance omissions.

Choose **Inspect declaration**, **Show candidate uses**, or **Follow written calls**.
Choose a nonempty source area before following calls. A call row preserves its callee
spelling and location; the possible destination buttons prefer that source area.
Inspect a call before trusting a same-name destination, especially for receivers, traits,
aliases or dynamic dispatch. An empty destination list is unresolved, not proof of no call.

Selecting a candidate extends a short trail. Choose an earlier trail step to return;
no branches expand automatically, so recursion does not create an unbounded diagram.
The order of written sites is syntax order, not execution order.

**Inspect call → Keep this location** uses the existing captured-source drawer. Keeps start
at the invocation's source line and retain a finite window; they do not save an entire
function or infer a relationship. Saved locations remain available beside optional notes
and reader-authored connections. **Open in VS Code** is explicit and verifies the working
file; it cannot inspect an unsaved editor buffer. Automated checks use a recording adapter.

After a source publication, find the area again before using stale results. Existing
saved locations retain their old bytes. Stop the host with Ctrl-C after the trial.

The [behaviour contract](../structure/PRODUCT.md), [architecture](../structure/TECH.md)
and [evaluation protocol](../structure/EVALUATION.md) explain the intended experiment.
Independent navigation benefit and any future small-model routing remain separate checks.
