# Choosing a map for repository orientation

This explanation is for contributors deciding how the directory map should help someone
learn unfamiliar code. It interprets research for this project's learning and review
workflows; the [behaviour contract](../prototype/ORIENTATION_PRODUCT.md) defines the
implementation. The first compact shelf map exposed directory labels but neither visible
containment nor control over grouping. Improving its aspect ratio did not solve that problem.

[Bederson, Shneiderman and Wattenberg (2002), Ordered and Quantum Treemaps](https://www.cs.umd.edu/~ben/papers/Bederson2002Ordered.pdf)
describe trade-offs between rectangle aspect ratios, item order and layout stability.
Their ordered algorithms retain useful sequence information rather than optimising
compactness alone; quantum layouts accommodate indivisible display objects. This supports
considering order and readable labels as design constraints, rather than treating a square
bounding box as sufficient. It does not validate our particular partition algorithm.

[Woodburn, Yang and Marriott (2019/2020), Interactive Visualisation of Hierarchical
Quantitative Data: An Evaluation](https://arxiv.org/abs/1908.01277) compared treemaps,
icicle plots and sundown charts with 12 participants after a pilot. Treemaps performed
worse on their navigation/hierarchy tasks, with a slight preference for icicles over
sundown charts. These were quantitative hierarchies, not source-review tasks. Their
consistent navigation framework also used overview/detail, zooming and breadcrumbs.
A treemap is therefore a candidate, not evidence that our repository view is intuitive.

[Bacher, Mac Namee and Kelleher (2016), Using Icicle Trees to Encode the Hierarchical
Structure of Source Code](https://arrow.tudublin.ie/scschcomcon/202/) studied a source
editor with an icicle overview. Their overview helped counting tasks but hurt locating
tasks. That is a useful warning against making the graphical map the only way to find
a file. Preserve path filtering and a familiar directory tree beside spatial navigation.

[Bouraffa, Fuhrmann and Maalej (2023), Developers' Visuo-spatial Mental Model and
Program Comprehension](https://arxiv.org/abs/2304.09301) compared a code canvas and a
conventional tab interface with 20 developers. Overall comprehension performance did not
significantly improve, although time spent in some activities differed. Personal spatial
organisation is plausible and requested by our maintainer, but is not an established
comprehension benefit that we can simply claim.

The resulting project choice is an ordered, nested directory map plus a directory tree,
with explicit collapse, focus/breadcrumb navigation and sibling reordering. Area represents
the currently expanded file/group units, not complexity or importance. Directory frames
show actual containment; no dependency or semantic subsystem is inferred. Compact unary
paths avoid wasting space on long chains. A visible depth limit leads to directory focus,
so deeper contents remain reachable without rendering arbitrarily deep geometry.

Reordering is presentation metadata scoped to a workspace. It never changes file paths.
Order and collapse survive filtering/refresh where the directories still exist; reset
returns to the default. All picking, overview and directory controls use one projected
scene. This choice directly addresses the reported friction, while leaving room for later
icicle, dependency or metric views when a concrete task justifies them.

Retest locating a known subsystem, inspecting an unfamiliar path and returning from a
review to related context. Check whether containment is understood, whether users can
predict collapse/reorder effects, and whether a directory remains findable after updates.
A scripted replay checks functionality and resource bounds. A maintainer's preference
and published studies do not substitute for an unfamiliar developer's task evaluation.
