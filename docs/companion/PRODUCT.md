# An investigation alongside the editor

This contract implements the maintainer's product-alignment decision after `0968fe4`.
The primary experience helps a reader explain how source fits together. It uses the
existing light/dark visual vocabulary; no separate design mock is supplied.

A developer starts with a question. While reading in VS Code, they add relevant source
locations to a bounded investigation: an entry point, a contract, an implementation or a
test. Each evidence card records the exact source observation, path, content/document
identity, line and a short captured excerpt. The developer supplies its title and note.
The cards retain context while the editor handles normal reading/navigation.

The developer can connect two cards with an explanation. Every connection is explicitly
reader-authored, not an inferred call/dependency/impact claim. A question, evidence and
connections form an explanation that can be revisited or exported. The first slice has
one investigation per configured workspace, at most 32 cards and 64 connections. It
starts empty; the lpc-rs oracle is not preloaded into the product.

The primary page is the investigation. Existing source browsing and review remain
secondary routes. Add-source controls are a temporary chooser, not a permanent file tree.
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
