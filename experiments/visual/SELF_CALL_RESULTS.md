# Source-local Rust self-call selection

For maintainers assessing the call-selection follow-up, 2026-11-08. The maintainer found
that choosing `self.render` in Minitosis opened unrelated same-name implementations.
The [selection explanation](../../docs/explanation/call-destinations.md) defines the new
source-local rule and its limits; the [guide](../../docs/how-to/follow-a-call-trail.md)
covers interaction. The original [prototype results](RESULTS.md) remain historical.

The Rust analyzer now records an optional destination ordinal and evidence kind for a
supported direct `self` call. The structural projection supplies that destination alone,
independent of directory ranking. The browser opens it directly and draws a solid **self
call** link. Unresolved receivers retain candidates and dashed links. No repository build,
new dependency, unsafe code or browser-side type guessing was introduced.

The [project-aware follow-up](SEMANTIC_RESULTS.md) extends field/alias navigation through
rust-analyzer; the results below describe the source-local-only implementation.

## Real-source check

A [functional replay](results/self-calls/result.json) on unchanged Tract Plugin Pack
`951cf85413029423e036c641638b845edafe75cf` now follows `process → render` with **zero chooser
entries presented**, versus four in the earlier replay. The actual destination is
`Minitosis::render`, from the same captured source. Caller DOM, rectangle and reading offsets
remain intact. Five fragments, one self-call link and three candidate links restore on
reload, with no page errors. The [source-bound document](results/self-calls/workspace.json)
and [overview](results/self-calls/working-set-light.png) retain the observation.

`self.processor.process_sample`, `self.processor.begin_block` and `self.dry.next` still
require choices: the adapter does not yet resolve field types/imports across files.
This is source-informed functional verification, not an independent navigation trial or
a claim of general Rust call resolution. Performance sampling was disabled in that replay
while mutation tests were running.

## Correctness checks

The initial core regression failed with three render candidates instead of one. The browser
regression failed on the earlier binary because the destination chooser blocked opening the
fragment. The final checks pass:

- Formatting and Clippy across all workspace targets, with warnings denied.
- 546 workspace tests including doctests. [Native log](results/self-calls/native-tests.log.gz).
- Complete browser suite, including direct opening, unknown-receiver choices, retained
  geometry, binding restore and a truncated one-item page that reports three total matches.
  [Browser log](results/self-calls/browser-tests.log.gz).
- Thirteen native binding tests cover inherent/trait implementation separation, lexical
  scopes, recursion, shorthand receivers/lifetimes, comments/turbofish, duplicate and
  conditional definitions, generic implementing types, partial syntax, legacy serialization,
  invalid destination metadata, mixed-language rejection and area-independent selection.

The binding collector visits the captured tree once, uses declaration ordinals from the
existing syntax walk and performs bounded metadata lookup afterwards. It shares the
amortised cancellation checkpoints. An intermediate refactor triggered the existing
cancellation-polling regression; the shared checkpoints restored that contract. Following a
call does not parse source or scan the repository.

## Mutation evidence

Every run used one low-priority worker, one compiler/jobserver task and serial tests, with
an unmodified baseline. Results are separated by candidate and scope:

| Run | Caught by failing tests | Survived | Did not compile | Timed out |
| --- | ---: | ---: | ---: | ---: |
| Initial native binding module plus changed validation/projection/hook code, 58 variants | 49 | 1 | 8 | 0 |
| Targeted concrete-generic regression follow-up | 1 | 0 | 0 | 0 |
| Final complete binding module after traversal refactor, 43 variants | 37 | 0 | 5 | 1 |
| Complete browser presentation model, 669 variants | 667 | 2 equivalent | 0 | 0 |

The initial survivor weakened the guard for concrete generic implementing types. A new
`impl Widget<u8>` case catches it; the final whole-module run also includes that case.
The final timeout removes the traversal's parent-exhaustion check, preventing progress;
it was stopped after 30 seconds and is **not** counted as an assertion failure. The five
unviable final variants are compile failures, not test coverage. Their logs and the timeout
log are retained in the [failure record](results/self-calls/native-mutation-failures.json.gz). The two presentation-model
survivors are the previously documented redundant view-ID anchor/link-ID type checks.
No exclusions were added. These results are evidence, not correctness proof or full-backend
mutation coverage.

A traversal run was interrupted after its baseline and four caught mutations while the
cancellation issue was corrected; it does not establish complete coverage. Raw records:
[initial native](results/self-calls/primary-native-mutations.json.gz),
[generic follow-up](results/self-calls/generic-followup.json.gz),
[interrupted traversal](results/self-calls/interrupted-traversal.json.gz),
[final native](results/self-calls/final-native-mutations.json.gz),
[presentation model](results/self-calls/model-mutations.json.gz).

## Indexing diagnostic

After correctness/mutation processes stopped, three sequential cold one-shot measurements
per size/binary compared the first repeated-lookup implementation with the final traversal.
Both used optimised native binaries, one analysis worker, fresh caches and one synthetic
Rust file containing a wide implementation. Medians in milliseconds:

| Calling methods | Initial implementation | Final implementation |
| ---: | ---: | ---: |
| 512 | 44.5 | 42.7 |
| 2,048 | 89.1 | 80.0 |
| 8,192 | 261.1 | 275.6 |

The [samples and binary hashes](results/self-calls/indexing-diagnostic.json) include process
startup, capture, parsing/colouring/binding and cache writes. This small diagnostic **does
not demonstrate an overall speedup** or qualify representative repositories/rendering.
The initial binary is an intermediate diagnostic build, not a released performance control.
The purpose is to expose indexing cost separately from interaction and correctness.
At 8,192 written calls the existing 4,096-call capture allowance applies; declarations are
still parsed. The [diagnostic script](measure-self-calls.py) records the source generator.
