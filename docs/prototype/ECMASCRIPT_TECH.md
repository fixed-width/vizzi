# JavaScript and TypeScript syntax adapters

## Context

[JavaScript and TypeScript browsing](ECMASCRIPT_PRODUCT.md) uses the existing
[source policy](SOURCE_POLICY.md), worker-owned [language adapters](LANGUAGES.md),
and structural call queries. The adapter receives captured bytes and a fixed
context. It can reuse a retained syntax tree for that context.

## Implementation

- Bind suffixes to the `javascript`, `typescript` and `tsx` registrations in
  `SourcePolicy`. Use that same policy to prune any `node_modules` path component
  during discovery and change handling.
- Select one grammar per worker-local parser. JavaScript and JSX share a
  grammar. TypeScript and TSX use distinct grammars.
  Giving each grammar its own context keeps the analyzer interface path-independent
  and prevents content-cache or retained-tree reuse across incompatible grammars.
  The qualification tool wraps the built-in factories from the same registry.
  [tree-sitter-javascript 0.25.0](https://github.com/tree-sitter/tree-sitter-javascript/releases/tag/v0.25.0)
  and [tree-sitter-typescript 0.23.2](https://github.com/tree-sitter/tree-sitter-typescript/releases/tag/v0.23.2).
- Keep declaration extraction and call rules in one adapter module. Use the
  same declaration classifier for facts and lexical scopes. For variable-bound
  functions, the declarator carries the name and encloses the body. Class methods
  use their written identifier or private identifier. Anonymous callbacks,
  object methods, computed/quoted method keys, and TypeScript bodyless signatures
  are outside the initial outline. Shared identity logic marks repeated keys as
  ambiguous.
- Allow the shared syntax collector to accept multiple expression rules so
  `call_expression` and `new_expression` use the same recorder, spelling limits,
  and nearest enclosing declaration. The adapter unwraps grouping and type-only
  syntax for callee names and function initializers. It keeps the written callee
  and the full declarator range for display and navigation. Destinations are shared
  name candidates. Syntax and call facts use the existing schema and routes.
- Future project-aware resolution belongs in the existing captured-source
  [provider interface](../visual/LANGUAGE_SERVERS_TECH.md). Keep configuration
  and import resolution out of the raw adapter so syntax facts stay cacheable
  by bytes and grammar.

## Testing and validation

- Exercise suffix selection, explicit exclusions, and nested dependency pruning
  during native capture (behavior 1).
- Check dialect-specific syntax, highlighting, exact UTF-8 name/range evidence,
  duplicate names, unsupported encodings and parser recovery at the adapter
  interface (behaviors 2–4).
- Check distinct cache entries for identical input by grammar, retained-tree
  isolation, targeted edits and rename, and equality with a clean capture
  (behaviors 1 and 4).
- Open a function bubble in Chromium and follow a candidate across language
  registrations using the native host (behaviors 2–3).

The Linux native CI check passed formatting, workspace tests and doctests, and
Clippy. All 15 tests in `experiments/browser/browser.test.mjs` passed in Chromium.
An optimized-host smoke test captured all 36 selected files in the installed
Playwright 1.64.1 package.

Cargo-mutants ran a passing baseline and tested 32 mutations in the adapter and
selected source-policy and call-rule code. Tests caught 30. Two replacements
requiring unavailable `Default` implementations failed to build. The run had no
survivors or timeouts. It excluded the equivalent replacement of
`Dialect::supports_written_calls` with `true`, since every dialect uses the same
nonempty call rule set.

Grouping and type-only wrapper regressions verify written spelling, exact
identifier spans, ownership, and unnamed computed callees. The Chromium trail
opens a wrapped TSX initializer from a TypeScript non-null call and follows
its grouped callee into JavaScript. The focused review run caught 10 mutations.
One replacement failed to build because a Tree-sitter node has no default value.
