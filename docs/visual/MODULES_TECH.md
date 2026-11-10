# Module identity architecture

Read [the behaviour contract](MODULES_PRODUCT.md) first. Module membership is source
metadata; spatial grouping and the optional contour are presentation state.

The capture policy retains Cargo.toml as source-only project metadata, with no execution
or repository build. It participates in immutable snapshot identity and ordinary watcher
reconciliation. A native module index consumes captured manifest text and captured syntax,
never mutable working files. Existing source-analysis fingerprints are unchanged except
for the normal dependency lockfile update.

Rust target roots come from captured Cargo package/target declarations and Cargo's default
root rules. Module edges come from parsed Rust declarations, literal path attributes and
literal module-scope include! inputs; lexical module parents refine each declaration.
Python files define modules, with captured __init__.py files establishing explicit package
chains. Classes/implementations are excluded from module paths. Conflicting contexts,
missing/capped/nonresident inputs and unsupported languages return no membership.

Rust module IDs bind the workspace, language, source root/target and qualified module path.
Python module IDs bind the exact module file; their labels reflect the captured explicit
package chain. Neither identity depends on source bytes, observation IDs or line numbers. The descriptor includes
language, label, root and provenance. Structural symbols carry this optional descriptor;
the native source/relationship refresh adopts it atomically with current source. A bounded
cache reuses Rust module-declaration extraction by captured version across observations,
retaining metadata only. Metadata queries never reopen private source backing.

Preparation admits at most 8,192 documents and 4,096 traversal states, Rust declarations
from files up to 1 MiB and an 8 MiB aggregate text budget, and manifests up to 128 KiB.
Only resident captured text is eligible. Reaching a limit yields neutral membership;
this is a bounded visual capability, not whole-repository module coverage.

Analysis-context upgrades can preserve a unique declaration's presentation identity by
reconstructing its earlier tracking hash from the current native lexical key, earlier
context reference and exact file identity. This never carries old facts or certainty into
the new observation. Only native unique keys are eligible; duplicate keys remain content
scoped. Optional file/context request fields let older clients fail conservatively.

The browser validates optional module descriptors and maps opaque IDs to a fixed palette.
The first six distinct modules in view order get distinct colours, probing past occupied
hash slots. Adding later views preserves those colours; removing an earlier module can
release its slot. Larger sets reuse colours and rely on labels and peer highlighting.
Readable labels/tooltips disambiguate roots. A small module presentation module handles
peer highlighting, header/minimap accents and optional outlines. Source-only DOM changes
do not affect call provenance or stored positions. The outline library is shipped locally
with its licence/version, with no network dependency in the viewer.

Contours use members and non-members from at most 24 visible bubbles. Coordinates are
normalised to a bounded working area before invoking Bubble Sets, then mapped back to
world coordinates. A geometry/module key caches results; pan/zoom reuse paths. Requests
are coalesced and disabled outlines incur no contour work. Failures/limits omit paths.

The contour treatment follows [Bubble Sets](https://vialab.ca/research/bubble-sets):
set membership is drawn over an existing layout. Readable labels and focus/hover cues
accompany colour, following [WCAG use of colour](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).
