# Native definition provider

For contributors implementing the [behaviour](SEMANTIC_PRODUCT.md). Preserve source
capture, parser facts, portable structure queries and presentation as distinct modules.

Captured call metadata gains an optional exact identifier span and a snapshot-bound call
identity. The native structure index retrieves the call witness by identity and maps a
returned definition position to the innermost admitted declaration. Never resolve by
terminal-name ranking when a provider supplies a location. Legacy missing spans cannot
manufacture an identifier position.

A host definition interface accepts captured workspace/call positions and returns semantic
locations or an explicit unavailable state. The first adapter owns one rust-analyzer process,
Content-Length JSON-RPC framing, request correlation, read-only server-request handling,
bounded messages/results and deadlines. It negotiates UTF-16 positions and uses standard
`textDocument/definition`, supporting Location and LocationLink responses. Language-specific
startup configuration stays in the adapter; editor integration can supply another provider.

A private mirror under external cache holds captured Rust documents and project Cargo
manifests/lock/configuration. Per-query synchronization changes only differing documents,
with didOpen/didChange/didClose notifications; source stays fixed throughout a lookup.
Cargo metadata stays offline, with build scripts, procedural macros and check-on-save disabled.
Cargo output goes outside analysed source; project wrappers are disabled. Project inputs,
server version/configuration and document versions identify the semantic context. Configuration
changes restart the session; source-only updates reuse it. Targets outside the mirror/admitted
snapshot remain unavailable to the source viewer. Native snapshot/configuration checks reject
superseded results. No unrelated source is substituted after a failure.

The browser can prepare analysis with one real unbound captured call after a function
opens. A small scheduling module admits one preparation at a time, discards destinations
and resets on capture changes. The foreground selection waits for pending work, then
requests a fresh definition; it never navigates using speculative results. Native requests
serialize within the host's bounded work admission, so concurrent preparation cannot produce
a busy name-match fallback. It keeps per-view request guards, displays pending work without
replacing the caller, and stores definition evidence with the visual link. A sole complete
destination opens immediately; a truncated page is not a unique result. Automated tests use
small real projects and fake protocol peers; user repositories remain read-only. Run focused
TDD, conservative mutation checks, workspace/doctest/Clippy/format checks and browser tests.
Measure warm/cold lookup cost separately after correctness/mutation workloads stop.
