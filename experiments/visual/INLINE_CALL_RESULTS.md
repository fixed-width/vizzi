# Inline call navigation validation

For maintainers assessing the source interaction, 2026-11-08. Call identifiers now
open a small action menu directly in captured source. **Open definition** follows the
existing destination rules; Ctrl/Cmd-click and Ctrl/Cmd+Enter follow immediately. The
[usage guide](../../docs/how-to/follow-a-call-trail.md) describes the controls and the
[architecture](../../docs/visual/TECH.md) describes the bounded metadata projection.

## Behaviour checked

The regression fixture failed against the earlier host because no inline call actions
existed. Native and pure range tests first failed on the missing interfaces, then passed.
The final implementation passes:

- Formatting and Clippy with warnings denied; workspace tests and doctests: **573 passed,
  3 opt-in real-provider tests ignored** by the normal workspace command.
- The complete browser suite, including the new source-range, DOM and native-host inline
  checks and the existing semantic warmup, destination selection and visual workspace checks.
- Exact UTF-8 identifier positions after Unicode, repeated names, wrapped receivers and
  identifiers at line endings; visible-section boundaries, paging and nested ownership.
  The projection works with unavailable source backing because it reads captured metadata.
- Syntax-span and source-text preservation, literal comments/strings, menu and shortcut
  navigation, Enter/Space/Down/Escape focus handling, source selection without navigation,
  stable caller geometry, reuse of open destinations, themes and disabled stale actions.
  A focused follow-up also checked Cmd-click and an unscaled menu inside a 320-pixel
  viewport with zoomed source.

The source renderer consumes captured identifier spans, rather than searching source text
for matching names. Clipped/overlapping or invalid ranges remain plain text. The existing
Calls list remains available. The menu currently implements only **Open definition**.

## Real repository replay

After the other checks stopped, the optimised host and isolated Chromium replay opened the
known `tract-plugin-pack` call sites with one low-priority Rust analysis worker. It used an
external captured-source mirror, disabled repository builds/procedural macros/check-on-save,
and launched no editor. The source repository was not modified.

| Interaction | Destination | Result |
| --- | --- | --- |
| Click `GsMeter::process` line 317's `process_sample`, then Open definition | `LufsMeter::process_sample`, `gs-meter/src/lufs.rs` | Unique definition; caller retained |
| Ctrl-click `Minitosis::render`'s `process_sample` | `Processor::process_sample`, `minitosis/src/engine.rs` | Unique definition; caller retained |

Both started with eight terminal-name candidates. No destination chooser was needed for
the language-server definitions. The replay checked caller DOM identity, position and
scroll state; it did not evaluate independent comprehension or usability.

With a fixed 12-second reading interval before each action, observed browser response times
were approximately **112 ms** and **159 ms** (the first includes menu interaction and a
screenshot). Repeated native HTTP lookups took approximately **4–8 ms**. These single-session
observations confirm the path works after background preparation; they are not a renderer
benchmark or latency distribution. The [cold-lookup investigation](COLD_LOOKUP_RESULTS.md)
records the remaining cold semantic-analysis cost.

## Mutation evidence

All runs used passing unmodified baselines, one mutant/compiler task and serial tests at
nice 10. The native scope covered the complete new `Structure::actions` method; the initial
diff scope found the same twelve mutants. The first run left a section-end comparison alive.
A behavioural test for the exclusive section boundary caught it on the follow-up run:
**8 caught, 4 unviable**, with no survivors or timeouts. The unviable replacements require
nonexistent `Page` constructors/defaults and are not counted as caught.

The complete new pure JavaScript range module had **83 caught, 3 surviving, 0 timeouts and
0 runner/build errors**. The three survivors are equivalent within this module's validated
range contract: removing the first separator after a fixed 64-character identity cannot
create key collisions; changing the end-column tie-breaker cannot change the rejection of
ranges sharing a start; and initialising the overlap endpoint at +1 instead of -1 leaves
all nonempty, nonnegative ranges with the same first-group endpoint. No mutants were
excluded. Mutation coverage does not extend to all DOM/menu event handlers or prove complete
correctness; those paths have browser regression checks.

Compressed logs, mutation outcomes, source revision/binary hash, replay results and screenshots
are in [results/inline-calls](results/inline-calls/checksums.json). The replay is reproducible
with the command below and a new output directory:

```sh
VIZZI_READING_MS=12000 VIZZI_RUST_ANALYZER=/path/to/rust-analyzer \
  nice -n 10 node experiments/visual/replay-definitions.mjs \
  /path/to/tract-plugin-pack /tmp/vizzi-inline-replay --inline --measure
```
