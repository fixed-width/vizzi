# Follow calls within a captured file

For readers exploring a historical function or a pinned comparison without mixing source
revisions. Use the existing call controls, candidate chooser and bubble design.

## Behaviour

1. History and comparison bubbles support inline call actions and the Calls list. Opening
   a destination reads its declaration from the caller's exact captured file and revision.
   Calls beyond the first loaded section work through normal paging. Written call labels,
   byte coordinates, analysis coverage and candidate limits retain their native meaning.
2. Supported source-local bindings can open directly. A sole complete same-file name
   candidate can also open, with candidate evidence preserved. Multiple candidates require
   an explicit choice. The UI states that lookup is limited to the captured file; it does
   not infer receiver types or claim runtime dispatch.
3. A missing destination remains unresolved. No lookup substitutes the working tree,
   another file, a dependency or live language-server evidence. Configured semantic
   providers continue to serve current-source navigation independently.
4. The destination opens as a pinned captured-source bubble beside its caller, inheriting
   its task group. It shows the actual revision/capture label. Source line numbers, reflow,
   indentation and copying remain literal. Earlier views, rectangles and zoom remain intact;
   reveal the destination using the normal call-trail camera behavior.
5. Following more calls stays in that same captured file. Revisits and cycles reuse a view
   in the same capture context, including recursion back to the original history or
   comparison bubble. Equal source text in another revision or live view is not permission
   to merge their reading contexts. Respect the 24-view and 96-link task bounds atomically.
6. Links identify candidate versus source-local evidence and their captured revision. They
   survive unrelated working-tree publications while both ends retain that capture. If
   either end changes revision or returns live, retain the relationship as historical;
   it never silently becomes current call evidence.
7. The new callee can close independently or Return to live. Missing current declarations
   retain the actual captured source until tracking succeeds. Editor, Save, callers and
   references remain current-source actions. A captured child does not invent its own
   function-history timeline or comparison from the caller's history selection.
8. Persist source bindings, layout, reading state and link evidence in tasks. Existing
   native owner expiry/restart behavior remains explicit; loaded source remains readable
   and no expired request falls back to current source. Following calls does not create
   another source archive or change the owner's capture quotas.
9. Closing views, changing source, switching tasks or newer editor/source navigation
   invalidates pending lookup and source preparation. Late successes and failures are
   silent. Source selection defers replacement. Invalid replies, expiry and resource
   refusal leave the working set intact rather than inserting half a destination.
