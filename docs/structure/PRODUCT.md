# Structural exploration experiment

## Summary

For developers learning unfamiliar source beside an editor. This opt-in experiment offers
an inventory of declarations and source-backed call-site exploration. It tests whether
explicit structural operations reduce navigation effort before adding another natural-
language classifier or model. The existing overview, review and question-search defaults
remain available. Existing visual styles and preview/editor controls are reused; no
separate mock is supplied.

## Behaviour

1. `/investigate?trial=structure` starts with searchable source areas. A reader can choose
   an area without knowing a path. The single search input finds names/areas; it does not
   promise to interpret a natural-language task or infer a domain-specific inventory.
2. Inventory enumerates captured declarations within the chosen area. A name search is
   retained when choosing its matching file; the full area inventory remains an explicit action. Show declaration
   kind, enclosing names, source location and counts of candidate call sites. Pagination
   and truncation remain explicit. A declaration with no recorded calls is not proven unused.
3. A reader can inspect a declaration, inspect candidate uses, or follow calls written
   inside it. Call sites retain their actual spelling and exact captured location.
4. Following a call shows possible declarations with the same terminal name, preferring
   the selected source area. Same-name ambiguity, dynamic/unknown calls, outside-area
   candidates and missing/limited analysis remain visible. Candidate links are not resolved
   calls, runtime order, data flow, or proof that an effect executes. No semantic path is
   invented across an unresolved step.
5. A short trail retains the reader's selected steps. Returning to a previous step restores
   its scope. A recursive or repeated target remains navigable without automatic expansion.
   The reader determines the path; the experiment does not generate an answer or call graph.
6. Existing captured previews and explicit location keeps/verified editor handoffs are
   available. Save controls require loaded investigation state. A kept location is the
   existing finite captured window, not a claim to retain an entire declaration. Call-site
   keeps start at the invocation so that the supporting spelling is retained.
7. Every result binds snapshot, document version and analysis context. A source publication
   invalidates the trail/actions and requires a new query. Old bytes never silently move to
   a different version. Errors outside the panes use the existing dismissible message surface.
8. Raw Rust/Python call syntax is supported initially. Other adapters/source-only/deferred
   documents report unavailable call analysis rather than zero use. Imports, traits, receiver
   types, macro expansion and runtime dispatch are not resolved by this slice.
9. Operations use bounded metadata and do not parse or execute the inspected project during
   navigation. No inference, source-control-specific identifiers or additional editor action
   is needed. The default theme follows the existing system/override setting; keyboard and
   narrow-screen behaviour follow the companion.
10. Development uses independent Rust/Python fixtures for inventory, call ownership, same-
    name collisions, dynamic calls, cycles, scope limits and source replacement. The two
    Tract questions remain evaluation-only. Sketchlayer and its reserved questions stay
    unread/unqueried until the candidate and development checks are frozen.

This is a capability experiment. Source-aware automated replay verifies function and
bindings. It does not prove comprehension or independent navigation improvement. Compare
unseen tasks with ordinary name/text search and count the required source inspections,
recording familiarity and order effects. Keep the natural-language layer unqualified if
these operations cannot provide a clear advantage.
