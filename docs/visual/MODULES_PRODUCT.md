# Module identity in the visual workspace

For readers orienting themselves across a call trail. Every bubble with known language
membership has a qualified module label and a narrow, shared header accent. The minimap
uses the same accent. Colour supports the readable label; duplicate labels are qualified
by their source root. Identity and colours remain stable across ordinary edits/reload.

Hovering or keyboard-focusing the module label emphasises its peers and minimap entries,
without moving views or changing zoom. This cue is distinct from the existing blue call
highlight. The label's specific hover target takes precedence over call highlighting.
Source content, copying, navigation, pinning and fitting remain available.

Membership comes from native language-aware analysis, not enclosing class names or generic
directory grouping. The first slice covers Cargo Rust targets and explicitly declared
Rust modules/includes, plus Python modules and explicit package chains. Unavailable,
unsupported or multiply bound membership remains neutral. Raw syntax membership makes
no claim about compiled configuration, imports, runtime identity or macro expansion.

Cargo manifests are immutable captured project inputs. Editing a manifest or a module
declaration republishes coherent module information with the source observation. Existing
live declaration tracking survives an analysis-context change only when its exact native
lexical identity can be verified; ambiguous/duplicate identities are never guessed.
Pinned and unresolved views retain their captured module context. Old saved layouts
remain usable; live views without module fields are hydrated through native refresh.
Legacy pinned views stay neutral until followed live; existing capture-expiry limits apply.

**Settings → Module outlines** enables an optional Bubble Sets overlay, off by default.
It outlines same-module sets behind bubbles without changing their positions. Non-members
are obstacles; blue call arrows and source always remain readable and interactive. Only
groups with two or more bubbles need an outline. Computation is bounded/coalesced, reused
for camera-only changes, and unavailable contours are omitted rather than replaced by
misleading boxes. The preference survives reload and follows the active working set.

Validate native membership/invalidation and conservative fallback with small fixtures;
test grouping, colours, overlay geometry, hover/focus, persistence and old layouts through
the model/browser interfaces. Use real source for a separate functional/performance replay.
