# An investigation alongside the editor

This contract implements the maintainer's product-alignment decision after `0968fe4`.
The primary experience helps an unfamiliar reader discover how source is organised,
then retain useful context. It uses the
existing light/dark visual vocabulary; no separate design mock is supplied.

Opening the landing page activates read-only indexing and automatically presents source
areas, file counts and languages from the captured repository. No question, known path,
line number or title is required to begin. The only initially visible text input is an
optional search across file paths and declaration names. Area clicks narrow the overview;
breadcrumbs return to earlier scope. Results are finite pages, with visible continuation.
Directories reflect actual containment, not inferred subsystem dependencies. Coverage
labels identify source-only and deferred files; the overview covers selected source,
not every repository document or build configuration.

A file or name opens a temporary source preview with a small set of captured declarations.
Selecting a declaration changes the preview's exact location. **Keep this location** saves
it in one click with a derived title; no form is required. Each saved location records
the exact snapshot, file/content/document identities, line and literal bounded excerpt.
Normal source reading and editing still belong in VS Code. Existing saved data remains
available through **Saved locations**; the landing overview does not overwrite it.

Questions and notes are optional. The question editor is disclosed on request in saved
context, notes only when inspecting one saved item, and connection controls only on request
when at least two items exist. Export and advanced path entry live under secondary controls.
The initial page must not show an empty investigation form or require the reader to
describe relationships before discovering any relevant source.

The developer can connect two cards with an explanation. Every connection is explicitly
reader-authored, not an inferred call/dependency/impact claim. A question, evidence and
connections form an explanation that can be revisited or exported. The first slice has
one investigation per configured workspace, at most 32 cards and 64 connections. It
starts empty; the lpc-rs oracle is not preloaded into the product.

The primary page is discovery with saved context alongside it. Existing source browsing
and review remain secondary routes. Advanced add-source controls are a temporary chooser,
not the default discovery mechanism or a permanent file tree.
A source preview is an on-demand evidence drawer; no full editor/outline pair dominates
the investigation. Preserve native/browser availability, theme and bounded-source rules.

Open in VS Code is an explicit action. The native host resolves the saved anchor and
checks current captured and actual working content before sending a local location to
the editor. Changed, missing, unsupported or unavailable source is refused with an
explanation; it never substitutes live bytes for saved evidence. Excerpts remain readable
if an old snapshot expires. A successful handoff verifies the source just before launch;
the editor opens a live working file, which can change afterwards. The CLI cannot inspect an existing unsaved editor buffer. A plugin handshake
would be needed to guarantee which bytes the editor eventually displays.

Persistence is optional via an explicit investigation-file launch setting outside the
source root and separate from disposable analysis cache data. When no file is configured,
the UI identifies session state and offers export. Atomic saves precede publication, so
write failure preserves the last saved investigation. Refreshing the source index updates
freshness labels without retargeting cards or rewriting reader notes/connections.

The selected learning workflow asks how lpc-rs obtains source, retries the c extension,
and reports a read failure. Read implementation/contracts/tests in the editor and use
Vizzi to retain evidence and explain the connections. Compare with an editor-only attempt:
what context was kept, what was reconstructed, and what did Vizzi add? A guided replay
checks function. Maintainer/unfamiliar-user evaluation remains separate.

First-slice exclusions: automatic dependency/call extraction, inferred impact, review
comments/approval, general graph layout editing, bidirectional editor plugin installation
and arbitrary revision handoff. Existing comparison/source adapters stay available for
future review evidence without introducing Git concepts into the portable investigation.
