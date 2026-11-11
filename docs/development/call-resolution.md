# Rust call-resolution implementation

For contributors maintaining the Rust receiver-binding and language-provider implementation.
The [user explanation](../explanation/call-destinations.md) covers what the results mean.
The visual workspace uses native analyzer evidence and an optional language-server definition provider. Directory
proximity and matching names alone do not identify the receiver of a method call.

The Rust adapter currently recognises a small source-local case: a direct `self.method()`
inside a non-generic `impl`, with one inherent method for that same type in the same lexical
scope and captured file. Both methods must use the same shorthand receiver form: `self`,
`&self`, or `&mut self`. Explicit lifetimes, whitespace, comments and method type arguments
are supported. The caller can belong to a trait implementation, as with
`Minitosis::process → Minitosis::render`.

The exact receiver restriction matters because Rust searches receiver types in order,
including automatic references and dereferences; a trait method on an earlier receiver can
precede an inherent method on a later one. See the
[Rust Reference's method lookup rules](https://doc.rust-lang.org/reference/expressions/method-call-expr.html).
This implementation recognises a matching inherent receiver without following that broader
search. It is deliberately narrower than compiler-backed Rust analysis.

Conditional/attributed definitions, duplicate methods, generic implementing types, typed
`self` parameters, syntax recovery and receivers such as `self.processor` or arbitrary
variables remain name candidates in the source-local rule. A configured definition provider
handles receiver types, imports and cross-file definitions using the project model. No repository build or dependency
execution occurs during this analysis.

The analyzer records an optional declaration ordinal and evidence kind on the captured
call. Model validation checks its language, coverage, owner and destination bounds/name.
The analyzer fingerprint includes the binding implementation, so incompatible cached
analysis is not reused. The structure projection translates that ordinal to a version-bound
symbol identity; queries do not reload source, parse it again or infer types in the browser.
If a destination was omitted by projection allowances, the query falls back to candidates.

A source-local binding supplies one destination and the `rust_self_receiver` evidence kind.
Selecting that call opens the fragment directly. Its current arrow is solid; inspection identifies it as a resolved call.
With a provider configured, other calls use standard LSP definition lookup before any
name-match chooser. A unique captured definition opens directly with a solid current
link; multiple semantic destinations require a choice. Failed/unsupported analysis offers
explicitly weaker name matches. Without a provider, sole complete name candidates retain
the earlier direct-opening behaviour; other candidates use the chooser and dashed arrows. A one-item response that reports more total
candidates does not bypass the chooser. Neither arrow claims observed runtime execution.

Definition lookup uses the exact captured identifier span, UTF-16 positions and a private
source mirror. Returned locations map to captured declaration identifiers, not enclosing
functions or matching names. Caller/target versions, source changes and project inputs are
checked; known external definitions do not produce unrelated local choices. The semantic
context is saved with the link. See the [provider contract](../visual/SEMANTIC_TECH.md).

The first adapter uses rust-analyzer with Cargo metadata offline, build scripts/procedural
macros/check-on-save disabled, and one worker. Dependency sources must already be available.
Macro-generated declarations, external source and unsupported project models may remain
unavailable. Standard [LSP definition lookup](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/languageFeatures/#textDocument_definition)
identifies declarations; it does not establish concrete runtime dispatch. The disabled
[project execution settings](https://rust-analyzer.github.io/book/configuration.html) remain
part of the adapter configuration.

The browser preserves the link kind with the layout. Earlier candidate links remain
candidates; upgrading the analyzer does not silently promote saved links or retarget their
source versions. The [call-trail guide](../how-to/follow-a-call-trail.md) covers interaction.
