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
  use their written identifier, private identifier or unescaped quoted name.
  Variable-bound object literals supply lexical parents for their methods.
  Objects, computed methods, anonymous argument callbacks and bodyless signatures
  can carry outline ranges without taking ownership of written calls. Shared
  identity logic marks repeated keys as ambiguous. Long computed names use a
  UTF-8 preview of at most 128 bytes and a digest of the complete computed key.
  The traversal reuses that digest for nested declarations.
- Allow the shared syntax collector to accept multiple expression rules so
  `call_expression` and `new_expression` use the same recorder, spelling limits,
  and nearest enclosing declaration. The adapter unwraps grouping and type-only
  syntax for callee names and function initializers. It keeps the written callee
  and the full declarator range for display and navigation. Destinations are shared
  name candidates. Syntax and call facts use the existing schema and routes.
- Route the three grammar contexts to the existing VS Code provider when
  sharing is enabled. The editor handles project settings, module resolution and
  type inference. The bridge applies the same saved-source and result checks
  as for other shared languages. Quoted methods record a checked opening-quote
  definition position alongside the exact unquoted identifier span.
  [Provider contract](../visual/LANGUAGE_SERVERS_TECH.md)
- Capture `package.json`, `tsconfig.json` and `jsconfig.json` as source-only evidence.
  Resolve package cues from the nearest captured ancestor manifest and give the cue
  one identity per workspace and manifest path, across the three grammar contexts.
  Historical lookup identifies the nearest eligible regular manifest in the selected
  Git commit and reads that one manifest with a 128 KiB limit.
  [Module identity contract](../visual/MODULES_TECH.md)

## Managed TypeScript server

The TypeScript profile uses the shared LSP client and `ProjectServer`. Opt in
with `--typescript-server` and an installed TypeScript 7 or newer `tsc`.
Vizzi runs it with `--lsp --stdio`, disables external code at initialization,
and disables automatic type acquisition through the server's configuration
request. Captured JSONC project configs are prepared in the mirror
with compiler plugins removed and original path mapping order preserved. The
reader accepts a leading UTF-8 BOM. Relative project-config references must
resolve to captured files. Captured package manifests stay in the mirror for
import resolution. A rejected config is represented by a safe placeholder with
its validated file selection settings. Its error applies when TypeScript selects
that project or a project that needs the rejected config for inheritance or
project references.

The profile provides LSP-specific language IDs for JSX and TSX. It sends source
opens before the lookup, using the native server's ordered document processing.
When the mirror contains rejected configs, `custom/projectInfo` identifies the
selected project with a bounded response before navigation proceeds. The semantic
configuration digest covers relevant captured paths and content, so a changed
source or project input starts a fresh server on the next query. This covers
cross-file re-resolution without waiting for asynchronous filesystem events.
External locations remain outside the capture.
[Native server command](https://github.com/microsoft/TypeScript/blob/main/tsc/cmd/tsc/lsp.go) ·
[Native server initialization and settings](https://github.com/microsoft/TypeScript/blob/main/tsc/internal/lsp/server.go)

The managed integration fixture uses `VIZZI_TYPESCRIPT_SERVER` to select the
installed tool. With that variable set, the native
`language_servers` suite checks frozen settings and package exports, aliases,
quoted definitions, references and cross-file updates. The Chromium
`browser.test.mjs` suite checks the standalone call and reference workflow.

Verified with the native TypeScript 7.0.2 executable and its npm launcher.
Closing the stdio client's input before reaping the launcher lets a wrapped
server exit on EOF. The npm launcher passed with Node.js 22.14.0, which uses
a child process for the native executable. The native workspace suite,
all 16 Chromium tests, and the Pyright and clangd browser smoke tests passed.
Four focused project preparation mutations were caught after a passing baseline.

Review regressions exercise equal-prefix alias precedence, BOM-marked JSONC,
and rejected configs that are unused, inherited, or referenced by the selected
project. A lookup in a file with 43,000 declarations passes with the same LSP
frame allowance. Native CI, all 16 Chromium tests, and the real VS Code
editor-provider test passed. The follow-up mutation run caught all five
selected changes to config rejection and dependency checks.


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
- Resolve a project alias and a typed receiver through the shared editor
  provider, and map definition and reference results to captured source
  (behavior 5).
- Check nested manifests, shared cues across grammars, distinct duplicate
  package names, manifest changes and commit-local membership (behavior 6).

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

The package and semantic follow-up passed native CI, all 15 Chromium tests and the
isolated VS Code extension-host test. The editor fixture uses a TypeScript
`paths` alias into JavaScript, discriminates same-name methods by receiver type,
and maps a semantic reference back to its written call. Chromium checks stable
package identity across a manifest rename. The native history fixture reads the
closest package in the chosen commit despite an oversized outer manifest. An
oversized closest manifest leaves the historical label neutral.

Focused mutations of package validation and method detection caught 24 of 27 in
the first run. Two replacements requiring unavailable default values failed to
build. The remaining mutation was caught on rerun after adding a destructured
object regression. Neither run timed out.

The review regressions verify that a quoted method accepts its opening quote
while written-call matching and reference positions retain the exact name. The
real VS Code fixture resolves a typed receiver to a quoted method and finds its
reference. A generated fixture with 512 nested functions verifies bounded
UTF-8 labels and distinct, compact identities for long computed keys with a
shared prefix. Native CI and all 15 Chromium tests passed.

The review mutation run caught 13 of 15 mutations. An invalid let-chain
replacement failed to build. The existing module inventory test caught the
coverage-count mutation on a targeted rerun. Neither run timed out.
