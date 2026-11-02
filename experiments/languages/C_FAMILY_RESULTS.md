# Raw C/C++ adapter evidence

Raw C/C++ adapters now use the existing source policy, analysis cache, immutable
documents, worker queue, and linked views. Default discovery remains Rust/Python;
C-family scopes require explicit enablement, and ambiguous headers require assignment.
CUDA and unassigned inputs remain source-only. The
[extraction reference](../../docs/prototype/C_FAMILY_SYNTAX.md) defines supported forms
and their limits; [the procedure](README.md#check-raw-cc-adapters) reproduces these checks.

## Implementation and correctness

C/C++ introduced a real variation in the internal syntax interface: one typedef can
declare several names, while type references and aliases should not create lexical
scopes. The shared traversal now accepts a reusable declaration buffer and separate
scope callback. Rust/Python keep their previous extraction behavior through that
interface. Cursor walking, recovery, key generation, and ambiguity handling remain shared.

Tests cover function definitions versus prototypes/function-pointer variables, every
typedef declarator, class/struct/union/enum definitions, using aliases, namespaces,
templates, methods, constructors/destructors, operators, qualified conversions, UTF-8,
grammar recovery, and unsupported encoding. Fixtures independently specify names,
source spans, and scope relationships. A failing regression fixture caught qualified
conversion names incorrectly including parameters/qualifiers; extraction was corrected.

Native tests verify explicit C/C++ headers, C++ inside a `.c` file, CUDA/LPC exclusions,
distinct cache contexts for identical C/C++ bytes, warm reuse, stable selection,
targeted edits, and cross-language rename/removal equivalence to clean captures.
The budget fixture mixes equal-size inputs from all four languages; barriers require
1, 2, and 4 simultaneous workers, counters enforce that shared limit, snapshots agree,
and warm hits create no parsers. The actual browser selects C definitions and C++
header methods, then preserves selection across a live position-only edit.

Formatting, **132 workspace Rust tests**, doctests, Clippy, and **33 Node/browser
checks** pass. New declaration fixtures first failed without C/C++ registration.
The direct dependencies are C grammar 0.24.2 and C++ grammar 0.23.4, verified with
`cargo info` and published latest documentation. Safe wrappers satisfy the existing
unsafe-code denial; no lint exception or analyzed-project tooling was introduced.

## Mutation evidence

The complete eight-module run passed its unmodified baseline and tested **233
mutations: 196 caught, 36 unviable, one timeout, zero survivors**. It covers C-family
extraction, shared traversal, registration, Rust/Python adapters, source policy, native
capture, and scheduling, using both core/native test packages and integration/CLI tests.

The timeout replaces refresh with no result and leaves initial publication/startup
waiting. It is separate from assertion-detected mutations, as are the unviable builds.
One existing equivalent directory-prefix comparison remains excluded; no new exclusions
were added. The [compact report](results/c-family-mutation-2026-11-01.json) records
commands, fingerprints, and outcomes not caught by assertions. Identity macro expansion,
grammar/dependency internals, OS delivery, and browser painting/glue are outside scope.
Browser production JavaScript did not change; its earlier mutation evidence remains
separate. Mutation results do not prove extraction completeness or compiler accuracy.

## Scoped native checks

These use one optimized binary, four shared workers, warm OS caches, and disposable
copies of pinned source archives. Each condition has three samples; every edit read
and parsed one file, and its snapshot matched a separate complete capture using the
same source-policy arguments. No tests or mutation jobs ran concurrently.

| Selected source | Captured / with analysis | Bytes | Empty analysis cache | Fresh-process persisted cache | One-file engine attempt | Write to coherent diagnostic response |
| --- | --- | --- | --- | --- | --- | --- |
| Linux `kernel/locking`, explicit raw C | 32 / 32 | 566,398 | 36.64–43.84 ms | 2.14–2.40 ms | 6.87–7.80 ms | 32.91–36.18 ms |
| PyTorch `c10`, explicit raw C++ | 420 / 413 | 3,023,982 | 183.75–220.57 ms | 12.53–19.87 ms | 2.63–3.20 ms | 30.55–33.20 ms |
| lpc-rs, default Rust/Python | 552 / 552 | 7,563,695 | 379.69–447.53 ms | 26.53–30.45 ms | 1.85–2.58 ms | 32.54–36.27 ms |

Kernel headers use an explicit root C assignment; PyTorch sample headers use an
explicit root C++ assignment. These are raw grammar selections, not build-derived
contexts. Linux has four syntax/28 partial files. PyTorch has 229 syntax/184 partial
files plus seven source-only CUDA inputs. Recovery is visible and does not establish
whether a compiler accepts a build. The default lpc-rs inventory contains 551 `.rs`
files and one `.py`; its `.c`/`.h` LPC fixtures never enter C-family analysis.

Live peak RSS was approximately 8.0 MiB, 20.8 MiB, and 34.5 MiB respectively on the
earlier i7-13700H / approximately 62.5 GiB host. Settled complete verification took
1.43 ms, 11.55 ms, and 11.54 ms. These are process observations, not memory limits.
The [native report](results/c-family-native-2026-11-01.json) preserves policy arguments,
binary hashes, source revisions, coverage/inventory summaries, source-manifest hashes,
and clean-capture checks. Both reports are based on `2e32dde` with implementation
uncommitted when measured.

## Qualification limits

The Linux/PyTorch runs are subtree smoke checks. They do not qualify full repository
memory, interactive performance during large C-family indexing, GPU presentation, or
the 16 GB laptop baseline. Native timings exclude browser rendering. Three warm-OS-cache
samples do not establish tail latency. Source-only and syntax recovery counts must
remain separate from semantic/build coverage.

The adapters do not expand macros, follow includes, resolve types/calls, instantiate
templates, infer header contexts, or enforce a compiler standard/target. Overloads and
conditional alternatives remain conservatively ambiguous. Full-scale native resource
qualification is the next [planned stage](../../docs/plans/c-family.md); configured
translation-unit analysis remains later work.
