# Originating-editor navigation

For contributors implementing editor adapters. Vizzi's native `Editor` seam delivers
verified captured locations to the exact window that launched it. The source host owns
capture, line and saved-byte verification; the editor adapter owns target-buffer checks
and presentation. Standalone launches retain their configured CLI editor adapter.

## Launch binding

The plugin creates a private per-launch loopback listener and descriptor outside captured
source, then passes `--editor-connection FILE` to its native child. This command-line-only
choice selects the originating-editor adapter ahead of standalone editor settings. It
never falls back to another window or CLI editor. No connection settings are copied by
the user, and browser requests cannot select a connection or executable.

```json
{"protocol":"vizzi-origin","version":1,"root":"/canonical/source/root","editor":"VS Code","session":"<64 hex>","port":12345,"token":"<64 hex>"}
```

Descriptors are regular files, at most 4096 bytes, with private Unix permissions. Native
startup reads and validates the descriptor once against its canonical root, then pins
its values. File replacement, provider sharing and other windows cannot redirect it.
VS Code gives a capable origin's session ID to its child in
`VIZZI_ORIGIN_JAVA_ARCHIVE_SESSION`. The launch descriptor keeps the v1 fields.
The unique launch descriptor has no root-wide latest-window alias. A new extension-host
instance requires a new Vizzi launch; an old association is not rediscovered.

The owned native workspace retains the listener through Stop Sharing, tab closure and
external-browser use. Launch failure, native exit, Stop Workspace and deactivation
dispose it. The semantic [connection protocol v1](BRIDGE.md) remains separate.

## One fixed operation

One TCP connection carries one UTF-8 JSON line and its reply, each at most 64 KiB.
Only `127.0.0.1` is supported. Session, credential and request identities are checked.
The only operation is displaying this verified destination; arbitrary command names
and browser-selected paths are not accepted.

```json
{"protocol":"vizzi-origin","version":1,"root":"/canonical/source/root","session":"<session>","token":"<credential>","request":"<64 hex>","path":"/canonical/source/root/src/main.rs","line":42,"source":{"bytes":12345,"sha256":"<64 hex>"},"expires_at_ms":1234567890000}
```

Paths are literal absolute project paths or admitted dependency paths. Lines are
one-based, column one. The witness hashes the exact captured UTF-8 bytes, including BOM
and line endings; it supplements existing native content identity. Preserve the existing
16 MiB project return limit and 2 MiB dependency admission limit. SHA-256 uses native
`sha2` 0.11.1 and Node's built-in crypto; no Node runtime dependency is needed.

For a Java archive, `path` is the canonical source archive. The request also
carries `providerUri` for the JDT document, `archiveBinary` for its local binary
JAR and the opaque `archiveCapture`. Native code verifies the archive member
before requesting presentation.

Successful replies echo the pinned session and request:

```json
{"protocol":"vizzi-origin","version":1,"session":"<session>","request":"<request>","status":"opened"}
```

Refusals use `status: "error"` and bounded `code`/`message` fields. Only one callback is
active per listener. Native requests have a ten-second overall deadline; editor
execution is bounded by the supplied expiry and an eight-second adapter allowance.
Closed or expired connections cannot select the captured line later. No automatic
retry is performed. If acknowledgment is lost after selection, report that navigation
could not be confirmed rather than claiming nothing opened.

## Editor verification and presentation

Resolve open documents by canonical file identity, including workspace-root aliases.
Refuse dirty or divergent target buffers across aliases; unrelated dirty files do not
block navigation. Read bounded regular-file bytes, check the witness and UTF-8 encoding,
and compare editor text with those bytes. The editor's BOM-free UTF-8 text representation
is accepted only when the verified saved bytes contain that BOM.

Load and verify the target before presentation. Show the document without a captured
selection, await that preparation, then recheck saved bytes, target buffers, expiry and
connection availability. With no further await, set selection and reveal the captured
line. Never save, reload or edit buffers. A tab can already be visible if its document
changes during preparation; refusal prevents the captured-line jump.

For a Java virtual document, VS Code rechecks the current attachment paths and
buffer witness before opening the tab and again before selecting the line. Both
checks also ask the launched host to reread the exact ZIP member through
`POST /api/dependency/verify-archive`, with only the opaque capture ID. This uses
the originating window's Java provider, even after Stop Sharing.

Use the selected editor's display name consistently in source views and saved locations.
Successful navigation is silent; the native browser error path owns refusal messages.

## Verification

Exercise two windows on one root, descriptor replacement, closed origin, target-only
dirty checks, aliases, edits during asynchronous preparation/reads, large project files,
external Rust files, deadlines, malformed frames and acknowledgment loss. Retain native
source-admission and standalone CLI tests. Installed-editor checks cover tab closure,
external browser, Stop Sharing and dirty-target refusal.
