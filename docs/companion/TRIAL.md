# Evaluate the editor-companion workflow

This how-to is for a facilitator. Use the existing pinned lpc-rs learning question and
source archive from the [trial kit](../../experiments/usability/README.md). Record the
operator's familiarity with the project and fixture. Keep expected paths/answers and
source-reading rules in facilitator material, not the participant's investigation.

The question is how the compiler obtains source, decides whether to try the c extension,
and reports read failure. Give the operator the same task and expected answer format
for either condition:

1. **Editor only:** read and navigate in VS Code using their ordinary tools and notes.
   Record how context/evidence is retained and what must be reconstructed after leaving
   a file. Do not forbid normal editor capabilities to manufacture a Vizzi advantage.
2. **Editor plus Vizzi:** read in VS Code; use the investigation to retain source anchors,
   interpretation and connections. Source preview is available for evidence checks, not
   a required substitute for editor reading. Record manual-transfer friction and any
   redundant work. Finish with an explanation linked to the evidence.

Use different comparable questions or counterbalance conditions across operators where
possible. Repeating the same question after an editor-only attempt teaches the answer;
a faster second attempt cannot establish a Vizzi benefit. A maintainer or reference-aware
agent replay establishes functionality only. An editor CLI recording proves arguments
and refusal semantics, not what VS Code eventually displays.

Assess the resulting explanation, evidence correctness, unsupported conclusions, missing
context and how much of Vizzi's state was useful while the editor was in focus. Ask what
the operator would retain, remove or replace with an editor feature. Fewer clicks, more
cards or a larger diagram are not acceptance by themselves.

Retain the question, source pin, chosen condition/order, operator familiarity, evidence
artifact and concrete friction. Separate manual connections from verified source facts.
Use a disposable checkout. Do not run the inspected project's build/scripts to complete
the source-reading task. No real editor window should be launched by automated checks
without confirming a separate display/session; an isolated data profile alone is not an
isolated desktop.
