# Bounded declaration-outline reference

The browser outline is a virtual list of declaration buttons. Its logical list is
independent of the DOM window. Rows have a fixed 48 px height; the window includes
visible rows and eight rows of overscan on each side, capped below 200 total controls.
A focused control may remain pinned outside the visible window. Extremely tall panes
are capped so viewport growth cannot create an unbounded render window.

The list retains normal buttons, `aria-current` selection, and one roving tab stop.
List items expose full `aria-posinset` and `aria-setsize`; the rendered subset is not
reported as the whole list. Arrow keys move logical focus, Page Up/Down move by a
viewport, and Home/End reach the actual first/last declarations. Enter/Space activate
the focused button through the same selection callback as clicks. Keyboard navigation
reveals only as much scroll as needed and does not move outer-page scroll.

Source inspection remains linked to reconciled selection from the backend. Clicking
the current declaration again still reveals its source. Focus is not optimistic
selection: moving focus does not replace the selected entity until activation succeeds.
Empty outlines have an accessible empty state and no invented declarations.

## Stable updates and work bounds

The outline builds an identity/index map when selected file identity or document version
changes. The additive `SourceView.version` key permits index reuse on selection/filter
requests and unrelated publications. Without a version, source-object replacement
conservatively rebuilds instead of assuming unchanged facts.

For a changed document, a surviving top-row identity anchors scroll. A surviving focused
identity retains its control. If removed, focus moves to the next surviving logical
neighbor, then a prior neighbor, then the empty viewport as needed. Background updates
do not steal focus from outside the outline. A selected entity that disappears is
cleared by the existing backend reconciliation rules.

Scroll and resize events coalesce into animation-frame work. The DOM contains only
the window and optional focus pin; row nodes are keyed so scrolling/publication does
not detach a surviving focused control. Labels are text, not HTML.

The current response still transfers the complete selected outline and source text.
Index construction on document changes remains linear in declaration count, and
source splitting remains linear in source size. This module bounds DOM work; it does
not claim paged source/outline transport or a memory-admission policy.
