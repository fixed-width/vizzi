# Vizzi domain language

Vizzi coordinates visual source context for learning and code review beside an editor.
These terms distinguish source identity from the reader's spatial organisation.

## Language

**Source workspace**:
The repository or directory whose source is captured and analysed, independent of its
source-control implementation.
_Avoid_: workspace without a qualifier when discussing a visual layout.

**Visual workspace**:
A persistent spatial arrangement of views, groups, associations and reading positions
for a task. One source workspace can support several visual workspaces.

**Fragment view (bubble)**:
A concurrently visible view onto a selected source fragment, such as a function or method,
with its source identity and context intact. The displayed view is distinct from a saved
short excerpt used as evidence.

**Task working set**:
The source fragments and other views relevant to one investigation or review, arranged
so the reader can compare and revisit them.
_Avoid_: unqualified working set alongside memory/admission discussions.

**Call trail**:
A reader-selected route through call sites and candidate destinations. A visual call
trail preserves earlier steps without claiming resolved calls or runtime execution order.

**Source relationship**:
A relationship supported by analysis and source provenance, with its kind and uncertainty
retained. A candidate call and a resolved call have different meanings.

**Investigation association**:
A reader-authored link explaining why views matter together. Spatial proximity or a drawn
association does not turn it into an established source relationship.
