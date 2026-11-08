# Structural prototype development checkpoint

For contributors checking the frozen candidate before unseen evaluation. Explicit source-area
and declaration inventories, same-name candidate uses and written-call traversal now work
through a portable captured-metadata index and `/investigate?trial=structure`.

Rust/Python collect calls in the existing syntax pass. Capture validates literal callee ranges
and declaration containment; unavailable/partial/omitted analysis stays explicit. Other language
adapters do not claim zero-call coverage. The index reads no source backing and retains no
native Snapshot Arc. Call destinations are candidate names, not semantic/runtime links.
The browser supports selected trail/back, bounded pages, previews and existing current-only
location keeps/recording editor handoff. A name filter stays applied when opening its file.

The independent fixtures cover Rust/Python, nested/dynamic calls, ambiguity/area preference,
cycles, supported/unavailable/partial metadata, exact limits, UTF-8 labels, empty/end pages,
backing unavailability, stale captures and cache refresh. No Tract or Sketchlayer source was
used as development input. The reserved Sketchlayer source remained unread before the
[candidate freeze](candidate.json); its questions remain in their original reservation.

Formatting, Clippy with warnings denied, 533 workspace tests/doctests, the full browser suite
and 20 QA helper files pass. The [light](results/development/light.png) and
[dark](results/development/dark.png) views were captured from the browser fixture. These
checks establish function, not navigation usefulness or natural-language understanding.

Core mutation testing generated 118 variants: 91 caught, two reviewed equivalents, 24 unviable
build failures and one timeout. The equivalent omission-count mutations affect a missing-
callee branch that the bundled required-function-field grammar contract cannot reach. The
Unicode clip division mutation becomes nonterminating; the 30-second timeout is separate
from a test catch. No variants were excluded from execution. Initial surviving boundary,
UTF-8, scope, pagination and target-order variants motivated stronger interface tests.
All four native adapter variants were caught after adding the post-replacement cache query.
Runs used nice 10, one mutant/compiler/jobserver task and serial tests. This is not a new
full mutation qualification of every parser, HTTP or DOM implementation.

[Core audit](results/development/validation/core-audit.json),
[native audit](results/development/validation/native-audit.json) and phase/baseline logs are
retained under `results/development/validation`. The next checkpoint follows the
[evaluation protocol](../../docs/structure/EVALUATION.md). Original/focused prose queries
and literal-seed text search are controls; structural operations get no hidden source-aware
seed advantage. Human/agent operation selection and later source audit are labelled, and
independent maintainer effort remains a separate acceptance check.
