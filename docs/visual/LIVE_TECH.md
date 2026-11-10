# Live fragment architecture

Read [the live behaviour contract](LIVE_PRODUCT.md) and [visual architecture](TECH.md).
The [refresh interface reference](LIVE_API.md) records protocol fields and limits.
The engine continues to publish immutable observations. The live presentation binds a
stable view ID to successive native declaration identities; it never rereads a current
file under an old version label.

## Native tracking and refresh

Structural symbols gain a `tracking` reference derived from Document.entity_id. File,
analyser context and lexical key establish unambiguous tracking. Duplicate keys remain
content-scoped. Existing observation-bound symbol/call IDs retain their meaning. Exact
current IDs hydrate older layouts; an expired layout without tracking stays unresolved.
Renames, cross-file moves and ambiguous changes require explicit reopening. Line movement
within the same lexical scope/file is tracked without name guessing in the browser.

`POST /api/visual/refresh` accepts one expected observation, at most 24 live view references
and 96 retained link descriptions. Pinned views are omitted. Identity/size/area validation
precedes source access; the request body cap is 192 KiB. Before/after publication guards
reject an obsolete transaction. The response carries fresh symbols, reading positions,
visible call actions, the first calls page, and source windows/header origins for changed
or missing display buffers. Unchanged documents require no source reads. Work shares the
existing native structural index and operation allowance; no parse or repository build
is started by refresh itself.
The cache is shared within an observation. A new observation reconstructs the structural
index on its first structural operation; the working-set bounds do not bound that
repository-wide cost. Large-repository refresh latency still needs separate qualification.

Reading hints are presentation-only, not function identifiers. For changed source, a
bounded 96-line neighbourhood matches one unchanged complete line. Missing/duplicate
matches reset to the declaration start. Window and style/reflow limits are unchanged.
Fresh call checks require a unique written spelling and current source-local evidence or
a reader-selected same-name candidate. Removed sites and unresolved/old semantic evidence
remain historical. Language-server results are re-established only by explicit navigation;
refresh does not stamp an old semantic context onto new source.

## Presentation transaction

LiveUpdates coalesces observations, aborts obsolete transport, retries failures with bounded
backoff, and defers commit during source selection or foreground source operations. View
binding/reading/link intent is checked again after preparation. A response is validated
before the model adopts its source bindings. The synchronous commit preserves geometry,
camera, view IDs and collapsed state. Retained unresolved or pinned versions have their
own snapshot and cannot use current-source actions until their bindings are current.

Changed source uses SourcePane with the mapped reading anchor and a fresh declaration
margin. Unchanged source retains its DOM and retargets call controls by exact native
name/position keys, replacing only version-bound action identities. This also preserves
selection in unaffected panes. Old menus are dismissed when their publication expires.
Historical arrows retain their old source witness, with a distinct label/style. A fresh
explicit follow can replace one unambiguous historical presentation relationship.

Workspace schema 1 accepts optional per-view snapshot/mode/state, optional symbol tracking,
and link freshness. Older layouts receive defaults. Two retained views may converge on the same current declaration
after following again; their view IDs and geometry stay separate, including on reload.
Layout writes are coalesced during interaction and flushed when the page closes.
Pinning is distinct from Keep and from native capture archival. Loaded bytes survive pinning; native history
expiry can prevent later paging, and reload does not recreate an expired source archive.
The normal exploration/review interfaces keep their own existing update policies.
