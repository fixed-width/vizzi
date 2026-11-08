# How call destinations are chosen

For readers and contributors understanding why a call opens immediately or offers choices.
The visual workspace uses language evidence supplied by the native analyzer. Directory
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
variables keep name candidates. Trait defaults, imports, cross-file type identity, overloads
and dynamic dispatch need further language capabilities. No repository build or dependency
execution occurs during this analysis.

The analyzer records an optional declaration ordinal and evidence kind on the captured
call. Model validation checks its language, coverage, owner and destination bounds/name.
The analyzer fingerprint includes the binding implementation, so incompatible cached
analysis is not reused. The structure projection translates that ordinal to a version-bound
symbol identity; queries do not reload source, parse it again or infer types in the browser.
If a destination was omitted by projection allowances, the query falls back to candidates.

A source-local binding supplies one destination and the `rust_self_receiver` evidence kind.
Selecting that call opens the fragment directly. Its solid arrow is labelled **self call**.
Unbound calls with a single complete candidate also open directly; multiple candidates use
the chooser and dashed **candidate** arrows. A one-item response that reports more total
candidates does not bypass the chooser. Neither arrow claims observed runtime execution.

The browser preserves the link kind with the layout. Earlier candidate links remain
candidates; upgrading the analyzer does not silently promote saved links or retarget their
source versions. The [call-trail guide](../how-to/follow-a-call-trail.md) covers interaction.
