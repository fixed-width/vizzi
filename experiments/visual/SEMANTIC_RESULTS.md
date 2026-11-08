# Project-aware call selection

For maintainers evaluating the field-call follow-up, 2026-11-08. The source-local rule
could not identify `self.lufs_meter.process_sample` in `GsMeter::process`. Extending name
ranking would not supply the missing receiver type. This change adds a host definition
provider and a first rust-analyzer adapter using standard LSP definition requests.

The [behaviour](../../docs/visual/SEMANTIC_PRODUCT.md),
[architecture](../../docs/visual/SEMANTIC_TECH.md),
[selection explanation](../../docs/explanation/call-destinations.md) and
[usage guide](../../docs/how-to/follow-a-call-trail.md) separate requirements, rationale and
instructions from this validation record.

## Implementation and provenance

Captured metadata now includes exact callee/declaration identifier spans. The browser sends
an observation-bound call identity, not a filename/guessed column. The host converts UTF-8
byte positions to UTF-16, asks the provider, and joins its definition locations to admitted
native declaration identifiers. Aliased names can resolve correctly; a local-variable
location does not become its enclosing function. Known external/unrepresented destinations
do not produce unrelated local choices. Multiple/partial semantic results retain choices.

The Rust provider starts lazily and owns one low-priority server with one worker. It mirrors
captured Rust bytes and project manifests/configuration under private external cache, leaves
the analysed repository untouched, and reuses that mirror/server for source-only updates.
Caller and target bindings, current snapshot and project-input checks prevent stale results
from acting. Browser guards also reject late replies after close or superseding selections.
Semantic-context identity is retained with definition links; version-1 layouts still restore.

Build scripts, proc-macros and check-on-save are disabled. Cargo metadata is offline and
project wrappers are disabled; build output is directed into the private mirror. The real
provider integration test replaces a working file after capture and still navigates the
captured version, updates a definition without replacing the mirror, verifies build.rs never
executes, and checks the mirror is removed on shutdown.

The configured project root currently needs Cargo metadata and cached dependencies.
External/sysroot sources use the installed toolchain/cache and are not admitted to the
source viewer. Macro/build-generated and unsupported declarations can remain unavailable.
Definition links identify language-server declarations, not proven runtime dispatch. Basic
browsing and explicit candidate fallback remain usable without a provider.

## Real-source replay

[The script](replay-definitions.mjs) uses unchanged Tract Plugin Pack revision
`951cf85413029423e036c641638b845edafe75cf`, known source locations and the public browser/
native interfaces. This is functional replay, not an unseen question or comprehension trial.

| Call | Earlier name candidates shown | Semantic result |
| --- | ---: | --- |
| `GsMeter::process`, line 317, `self.lufs_meter.process_sample` | 8 | one definition: `LufsMeter::process_sample`, `gs-meter/src/lufs.rs:290` |
| `Minitosis::render`, `self.processor.process_sample` | 8 | one definition: `Processor::process_sample`, `minitosis/src/engine.rs:89` |

Both open directly with no chooser. Caller DOM, geometry and reading offsets remain intact;
no page errors were observed. The known-source replay does not establish complete Rust
resolution, dynamic dispatch, cross-platform qualification or independent usability.

The installed rust-analyzer is 1.99.1 (2026-10-13). The first standalone-file probe could not
resolve the field call; project Cargo information was necessary. Enabling Cargo autoreload
also mattered during initial project discovery. Three missing locked dependency sources
were fetched through `cargo metadata --locked`, without building the repository. No analysed
source/manifest was changed. The new url dependency is 2.5.8, verified with `cargo info`.

## Verification scope

Focused TDD first reproduced missing captured identifier spans, missing UTF-16 conversion and
the unavailable destination. Normal tests cover protocol bounds, UTF-8 framing, read-only
peer requests, response correlation, deadlines, alias targets, source changes during lookup,
known outside destinations and bounded semantic counts. Real rust-analyzer integration is
explicitly ignored in the normal workspace suite and run separately. The normal browser
suite uses a deterministic definition-response peer; `npm test -- --semantic` runs the actual
language server on a small project.

Mutation checks use one low-priority worker/compiler task and serial tests. Scope is the
UTF-16/byte conversions, bounded message reader/location decoding and the presentation model.
The full mirror/session implementation and complete controller/backend are not mutation
qualified. Timeouts/build failures/equivalent survivors are recorded separately from
assertion failures; results are evidence, not correctness proof.

Final checks passed: formatting, Clippy with warnings denied, 556 native workspace tests
including doctests, and the complete browser suite. Two real-provider integration tests are
ignored in the normal suite and passed explicitly, as did the real-server browser test.
[Native log](results/semantic/native-tests.log.gz),
[browser log](results/semantic/browser-tests.log.gz),
[real provider log](results/semantic/real-provider-tests.log.gz),
[real browser log](results/semantic/real-browser.log.gz),
[Clippy log](results/semantic/clippy.log.gz).

The focused native mutation run exercised 47 UTF-16/protocol variants: 34 initially caught,
10 initially surviving and three compile failures. Added independent end-of-line and exact/
combined message/header/result-limit tests catch nine of the ten survivors in the iterative
follow-up. The remaining match-guard variant still rejects primitive JSON through mandatory
location-field validation and is equivalent for valid results. Across the original cohort,
43 are caught by failing tests, one is equivalent and three are unviable. No timeouts or
exclusions. [Initial record](results/semantic/native-mutations-initial.json.gz),
[follow-up](results/semantic/native-mutations-followup.json.gz).

The complete presentation model mutation check caught 712 of 717 variants, with five
redundant validation/identity-branch survivors, no errors/timeouts and no exclusions. The
remaining variants remove a redundant evidence-tag check, force an already-equivalent
semantic-context branch for non-semantic records, remove the view-ID leading anchor, or
remove the link-ID type check. Conflicting context/evidence fields are rejected independently;
non-semantic records cannot carry a context; numeric view-ID and canonical link-ID validation
retain their invariants. A frozen version-1 workspace fixture also verifies backwards
compatibility. [Model report](results/semantic/model-mutations.json.gz).

## Final replay and lookup observations

The [final record](results/semantic/result.json), [GsMeter view](results/semantic/gsmeter.png)
and [working set](results/semantic/working-set.png) were captured after other test/mutation
processes stopped. The first GsMeter browser request took 10.48 seconds including private
project/server startup and initial semantic work. Three repeated native requests took
4.0–7.1 ms. The subsequent Minitosis browser request took 175 ms; repeated native requests
took 4.2–5.7 ms. These are one-session observations, not latency distributions, isolated
inference costs or large-repository qualification. Cold project startup remains a visible
cost; the caller remains readable while the lookup is pending.

Reproduce the functional replay with an optimised host and a new output directory:

```sh
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 node experiments/visual/replay-definitions.mjs /path/to/tract-plugin-pack /tmp/new-semantic-replay
```

Add `--measure` only when correctness/mutation workloads have stopped. To run the optional
real provider checks: `cargo test --offline -p vizzi --test rust_definitions -- --ignored` and
`npm test -- --semantic`. The Unix protocol-peer unit test additionally uses Python 3.

Completed mutation summaries and validation artefacts are compressed. Scratch mirrors,
mutation outputs and previous development/test compiler artefacts are disposable; the live
viewer retains only its active private language-server workspace.

After validation, development/test compiler variants were removed with
`cargo clean --profile dev`; the optimised viewer and installed browser dependencies were
kept. The checkout is about 934 MiB, with no private participant notes or recorded earlier
research removed. Completed scratch analysis/mutation workspaces were discarded after
archiving their records.
