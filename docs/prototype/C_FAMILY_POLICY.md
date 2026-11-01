# C/C++ source and analysis policy

Status: design contract for the next implementation slices. The running host still
supports Rust/Python as described in [LANGUAGES.md](LANGUAGES.md). This document does
not enable C/C++, import compilation databases, or describe implemented CLI options.
The [rationale](../explanation/c-family-analysis.md) connects these decisions to the
reference corpora and literature; the [delivery plan](../plans/c-family.md) defines gates.

## Scope and terminology

A **source policy** decides what to capture and why an analyzer is assigned. A
**language adapter** performs analysis at a requested capability. These are separate
interfaces: a file extension is selection evidence, not proof of its compiled language.

**Raw syntax** examines captured bytes without expanding macros, resolving includes,
selecting a build configuration, or invoking project tooling. A **compilation profile**
records one translation unit's compilation environment. **Configured analysis** uses
a selected profile and captured dependencies; its results belong to that profile.
Neither a raw parse nor one configured parse represents all program variants.

The next delivery adds raw C/C++ declarations. Compiler-backed resolution, import/type
relationships, CUDA analysis, and variability-aware parsing are separate later work.

## Capture and assignment

C-family discovery is enabled for explicitly selected workspace-relative subtrees.
Existing Rust/Python discovery remains the default. Enabling a family subtree exposes
candidate files, including source-only headers and unsupported dialects. It does not
assign every file in that subtree to one grammar.

Selection is deterministic and independent of directory iteration order:

1. Root exclusions, symlink rules, and explicit path exclusions take precedence.
2. Within an enabled family subtree, recognized unsupported dialects remain source-only.
3. An explicit C/C++ language assignment selects a raw grammar for matching supported
   candidates. The most specific path scope wins; conflicting assignments at the same
   scope are configuration errors. Matching uses complete path components.
4. If explicitly selected by the user, an imported profile's recognized language can
   provide the assignment. This is language evidence only until configured analysis runs.
5. Otherwise use the following case-sensitive candidate table.

Scopes use normalized, UTF-8 workspace-relative paths, including an explicit root
scope. Reject traversal and absolute source-rule paths; preserve existing symlink and
root-exclusion behavior. Normalize redundant equivalent rule encodings before hashing.
Unsupported dialect evidence includes a selected profile explicitly declaring CUDA,
even when the filename ends in `.c` or `.cpp`.

| Candidate suffix | Raw assignment within an enabled family subtree |
| --- | --- |
| `.c` | C, with extension provenance |
| `.cc`, `.cpp`, `.cxx`, `.C`, `.c++` | C++, with extension provenance |
| `.hh`, `.hpp`, `.hxx`, `.h++` | C++ header syntax, with extension provenance |
| `.h`, `.H` | Source-only: C/C++ language is unassigned |
| `.cu`, `.cuh` | Source-only: CUDA analysis is unavailable |
| `.inc`, `.inl`, `.ipp`, `.tpp` | Source-only textual fragment unless explicitly assigned |

An explicit assignment can override a supported suffix hint, for example C++ for a
`.c` input. It cannot override an exclusion or silently turn a recognized CUDA input
into ordinary C++. Other extensions remain outside this delivery. `.lpc` is excluded.
LPC can also use `.c` and `.h`; the lpc-rs reference profile never enables C-family
analysis. Mixed repositories can exclude LPC paths before any suffix/profile assignment.
Do not identify an LPC subtree by repository name or by trying several grammars.

A header is never classified by its nearest `.c`/`.cpp` filename, the repository's
majority language, or whichever grammar reports fewer errors. Raw assignment remains
explicit when evidence is ambiguous. Missing profiles do not trigger compiler probes.

| Example | Required result |
| --- | --- |
| lpc-rs reference profile, `tests/fixtures/mudlib/std/room.c` | Outside C-family scope; no C analysis |
| Enabled subtree, `src/a.c` | Raw C, extension provenance, unknown build coverage |
| Enabled subtree, `include/shared.h`, no assignment | Captured source-only, language unassigned |
| Same header with an explicit C++ assignment | Raw C++ with rule provenance; no preprocessing claim |
| Enabled subtree, `kernel.cu`, a general C++ assignment | Source-only CUDA; assignment does not erase dialect |
| `a.c`, explicitly selected profile declaring C++ | Raw C++ language hint; configured results require a separate analysis |

## Source-only files and visible coverage

Capture source-only candidates once, with classification provenance and a structured
reason such as `language_unassigned`, `dialect_unsupported`, or `fragment_unassigned`.
Their source is navigable without an invented C/C++ analysis record. Unsupported
encoding is a separate reason; it must not masquerade as an unassigned language.

Overview counts distinguish captured files from files with analysis, and filtering
includes source-only candidates. The inspector identifies raw language, assignment
provenance, grammar recovery, and configuration coverage. A header without assignment
shows an explanation beside its source. Configured contexts remain separate selectable
outlines; flattening them must not imply that all declarations coexist in one build.

Keep grammar recovery (`syntax`, `partial`, `unavailable`) separate from analysis mode
and build-configuration coverage. A clean raw tree still has unknown configuration
coverage. A failed configured analysis does not erase the usable raw source/outline.
Classification metadata and new counts require an explicitly versioned host response;
the existing schema-1 meaning must not be silently repurposed.

## Raw facts and preprocessing

Initial C declarations cover function definitions, named structs/unions/enums, and
typedefs. Initial C++ declarations additionally cover namespaces and named classes;
function definitions include supported methods, constructors, destructors, and operators.
Templates and overload fixtures must establish actual supported extraction before a
release claims those forms. Prototypes, variables, fields, and resolved relationships
are outside this initial outline contract.

The grammar version and extractor determine supported syntax. Raw mode does not claim
to enforce `-std`, a GNU dialect, a target ABI, or a compiler's defaults. Unsupported
forms and recovery remain visible, rather than being repaired by silently selecting
another grammar or rewriting the source.

Raw extraction can encounter declarations in different conditional branches. They are
lexical candidates, not confirmed active declarations. Do not evaluate `#if`, expand
macros, remove inactive branches, or claim that a raw walk parsed every valid variant.
Build activity remains unknown, including for literal `#if 0` in this initial mode.
If conditional text is retained later, it is source evidence until a capable backend
evaluates it. Duplicate qualified names, including unresolved overloads, retain the
existing conservative ambiguity behavior. Macro-generated declarations may be absent.

Facts keep byte spans in the original captured source. Include directives, if exposed
later, start as unresolved textual evidence; raw mode never follows them. Header edits
therefore do not invalidate another file's raw declarations unless a future extractor
explicitly consumes that header. Configured dependency invalidation is different.

## Compilation profiles: later configured analysis

The first import format will be an explicitly selected `compile_commands.json`.
Keep each distinct record for a file; do not choose the first or last command. Exact
duplicate records may share a profile identity while retaining their provenance.
Importing a database neither executes its commands nor enables compiler-backed analysis.
The format permits multiple records per input, and defines paths relative to each
record's working directory. [Clang's format specification](https://clang.llvm.org/docs/JSONCompilationDatabase.html)
is the import reference.

Retain original arguments, working directory, main input, optional output, database
identity, and interpretation diagnostics. Prefer the `arguments` array. A `command`
string needs a declared platform-specific decoder with no shell expansion; unsupported
quoting is an explicit unavailable profile. Do not rewrite `gcc` into `g++`, sort flags,
discard repetitions, or infer a driver through an unknown wrapper. Recognized `-x`
flags take precedence over suffixes when interpreting a selected profile.

Effective profiles preserve ordered include paths, macro defines/undefines and forced
includes, requested language/standard, target/sysroot/resource roots, toolchain identity,
and consumed response-file content. Unknown options or missing inputs prevent claims
of complete configured analysis. Record relevant preprocessing/ABI flags conservatively;
dropping an unrecognized flag to make parsing succeed is not profile equivalence.

Reading a database is data import. Launching builds, generators, wrappers, toolchain
probes, or repository-dependent tools remains an explicit product action under
[DESIGN.md](../../DESIGN.md). A later backend uses its own supported analysis invocation,
not arbitrary imported command execution. Response files and SDK/generated inputs must
be captured through declared native input roots and included in validity checks.

## Headers and multiple compilation environments

Raw header syntax and configured header meaning are distinct results. Configured
header analysis belongs to a translation unit/profile that includes it, with that
unit's preprocessing state and include order. If no such environment is available,
keep raw/source-only coverage. A matching basename is not sufficient evidence.

One header may have C and C++ includers, multiple targets, or different macro states
within one language. Preserve separate contexts and report which have been analyzed;
an on-demand selected context does not imply exhaustive enumeration. Opening one header
must not eagerly schedule every includer/profile combination. Provenance retains the
owning translation unit, profile, and source locations across macro/include boundaries.

Kernel profiles include the selected architecture, configuration, generated headers,
and build-directory inputs. PyTorch profiles keep host C++, C, and CUDA/device contexts
distinct. An unavailable build configuration leaves raw exploration usable; it must
not silently substitute the host machine's target or missing generated files.

## Identity, reuse, and invalidation

Introduce an explicit capture-policy identity before expanding discovery. It hashes a
versioned, deterministic representation of enabled scopes, assignments, exclusions,
and selection rules. Snapshots retain it, including empty snapshots. A policy mismatch
forces complete discovery before retaining unread documents. This closes the current
native-caller precondition that document-only context checks cannot detect scope expansion.
Workspace/file identities remain stable when only the source policy changes.

Classification and its provenance participate in immutable document versions; the
snapshot identity includes the capture-policy identity. Pinned queries retain their
original source, classification, and context. Equivalent policy encodings have the same
identity after documented normalization; meaningful precedence differences do not.

Raw cache records remain path-independent and keyed by bytes and analyzer context.
Changing an assignment can reuse a matching raw result, while its document records
the new provenance. Changing the language or extractor context cannot reuse the other
language's record, even for identical bytes.

Configured analysis must not reuse a header result using only that header's bytes.
Its validity envelope includes translation-unit identity, interpreted profile/toolchain,
captured source and dependency versions, and path-sensitive lookup state. Header edits,
newly appearing include candidates, changed search-path order, response files, generated
inputs, and SDK changes can invalidate results. Until dependency/lookup tracking is
complete, use conservative translation-unit or configured-scope reconciliation. A
future configured backend receives an immutable captured input bundle through a new
interface; the existing byte-only `Analyzer` must not start reading mutable includes.

## Resource limits

All languages and capability levels share the native analysis scheduler. Keep the
current four-worker default for initial qualification; importing many profiles must
not multiply the worker limit. Eagerly parsing both C and C++ for every header is not
part of this policy. Configured expansion needs bounded admission before activation.

The present worker limit is not a memory budget. Measure full captures while retaining
the prior snapshot, large generated headers, cache loading, cancellation between jobs,
and source queries during indexing. Publish candidate/captured/analyzed counts, bytes,
peak RSS, cold/warm timings, and edit timings separately. The 16 GB laptop target
requires qualification with the actual adapters; previous grammar-only timings do not
establish application memory use or an acceptable interactive budget.
