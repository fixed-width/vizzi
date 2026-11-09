# Fit source fragments without rewriting code

For contributors extending the visual workspace's source presentation across languages.
The reader needs useful code in a narrow pane while preserving source locations and
neighbouring fragments.

## Research and the Vizzi interpretation

[Bragdon et al., CHI 2010, page 2505](https://www.eecs.ucf.edu/~jjl/pubs/p2503-bragdon.pdf)
describe syntax-aware display reflow around commas, parentheses and operators. Widening
a bubble reveals its original layout; reflow does not edit the file. Their Java evaluation
supports the working-set interaction, rather than establishing results for every language
or for Vizzi's browser implementation.

**Vizzi design inference:** source identity should remain independent of its displayed
line breaks. A language-aware hint provider and a shared width planner let different
languages retain their own token boundaries while using the same display and navigation.

## Separate syntax policy from display width

The Rust engine projects optional break hints from an already captured source line and
its parser-derived styling. The native module handles language profiles, longest compound
operators, delimiter anchors, comment words, and protected tokens. Broad style captures,
including attributes and types, remain indivisible so hidden nested literals cannot be
split. This is a conservative lexical layout policy; it does not claim to reproduce a
language formatter's complete grammar or style rules.

The browser receives byte offsets and preferences, rather than language names or a new
formatted source string. Its planner balances structural breaks against unnecessarily
short display rows, chooses hanging indentation that leaves useful width, and keeps
indivisible tokens intact. A greedy planner keeps the initial implementation bounded;
this is not an optimal-layout or minimum-height guarantee. A future hint provider can
supply more precise syntax without changing the source-pane interface.

Layout uses the actual font and pane width, before camera transforms. Panning or zooming
the canvas therefore does not reformat source. Resizing coalesces through the browser's
animation frame and changes only affected line layouts. Source-window queries opt into
hints once; resizing and changing layout mode do not request source or run a formatter.

A function fragment removes its enclosing file indentation from the display. The captured
declaration line supplies a fixed baseline, including when a later section is restored.
Tabs are interpreted at four-column stops and internal indentation is retained relative to
that baseline. Original whitespace remains in hidden DOM text; decorative width represents
the remaining columns, so copying and call byte offsets retain the original source. Reflow
uses those displayed columns for width and hanging alignment. This differs from removing
the smallest indentation in whichever body section happens to be loaded.

## Preserve the reader's context

Every logical row retains its physical source line number. Continuations have a separate
mark and display indentation; these are not new source lines. Styling and clickable call
identifiers retain their captured byte positions across display segments. Copying a
selection within fully captured rows reconstructs only original characters and line
terminators, excluding line numbers and display decoration. Clipped selections retain
normal browser copying, including the visible omission indication.

A reading anchor records a source line, the byte at the visible segment's start and its
vertical offset. It survives width/mode changes and reloads. Older saved pixel positions
are interpreted using the original line layout before applying reflow. The presentation
model accepts the optional anchor without changing source identities or expiring existing
workspace documents. A resize waits while source is selected, preserving the selection.

## Keep limits explicit

A window still contains at most 96 physical lines and 32 KiB of captured text. The optional
projection admits at most 256 break points per line and 1,024 per window, atomically per
line. The browser limits planning to 4 KiB per line, 64 display segments and 4,096 distinct
font measurements. Exceeding an allowance retains the original line rather than dropping
text. Unknown language profiles, unavailable/limited styling, left-clipped source and
unavailable font measurement likewise retain original lines. Long protected tokens may
still need horizontal scrolling. The existing source-window omission markers remain.

These limits bound active work; they are not frame-time qualification. Correctness and
mutation checks use small fixtures; representative optimised-host/browser measurements
are recorded separately. See the [visual architecture](../visual/TECH.md) and
[usage guide](../how-to/follow-a-call-trail.md).
