# JavaScript and TypeScript browsing

Open a mixed repository and read JavaScript and TypeScript alongside its other
captured source. Readers can find functions and follow written-call candidates in
the existing call workspace without installing project dependencies.

## Behavior

1. Default capture includes `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.mts`, `.cts` and
   `.tsx`. JSX works in JavaScript and TSX. Standard TypeScript files use the
   TypeScript grammar, including angle-bracket assertions. Existing source
   exclusions apply, and capture skips `node_modules` at every depth.
2. Source Browser shows syntax colors and a declaration outline. It includes
   named function declarations, functions assigned directly to a variable
   (including through parentheses or TypeScript's type-only wrappers),
   named classes and object literals bound directly to a variable, and their
   identifier-named and simple quoted methods. It also shows computed methods,
   anonymous functions passed directly as arguments, and TypeScript interfaces,
   enums, type aliases and named bodyless signatures. Function ranges retain the
   exact saved text, including a variable's name and initializer when it supplies
   the function name. Nested names are qualified by the enclosing declarations in
   the outline.
3. **Add bubble** can find those functions. **Calls** records explicit call and
   constructor expressions and the written identifier on simple or dotted
   callees, including through parentheses and TypeScript's type-only wrappers.
   Computed callees stay unnamed, and JSX tags alone create no call
   evidence. Calls use the existing same-name candidate behavior, including
   ambiguity and missing destinations. Computed method names, anonymous callbacks
   and bodyless signatures appear as outline entries. The named implementation
   is the written-name candidate for an overload when a body exists.
4. Edits use the existing captured-source refresh. The source remains browsable
   when parsing recovers or the input encoding makes syntax unavailable. Raw
   navigation works offline and requires only the captured source.

5. From a trusted VS Code workspace with provider sharing enabled, JavaScript
   and TypeScript calls and references use the editor's configured language
   service. Returned positions must match captured saved source. Aliases and
   typed receivers can lead to a definition with a different written name or in
   a different captured file. Unavailable or unmapped definitions follow the
   existing semantic status and candidate rules.
6. A bubble with complete syntax coverage shows the name from its closest
   captured `package.json`. JavaScript,
   TypeScript, JSX and TSX in the same package share an accent. Two manifests
   with the same name have different identities. A closest manifest with an
   unavailable or invalid name produces a neutral bubble. Live changes update
   labels with the source observation. Historical labels use the selected commit.
7. A standalone launch can opt into the native TypeScript language server by
   selecting an installed TypeScript 7 or newer compiler executable. The server
   supplies definitions and references for the captured JavaScript and TypeScript
   files without an editor connection. Captured project configs and package
   manifests determine project settings and import aliases. Supported settings
   include project-local inheritance, project references, and path mappings.
   Unsupported settings required to open the project report unavailable
   semantic analysis, and the existing written-name evidence remains available.
   The server uses
   captured source and settings across edits, and project plugins and automatic
   type acquisition are disabled.
8. Managed and editor-owned definitions can open a reached file from a locally
   installed JavaScript or TypeScript package. Package source follows the existing
   dependency capture contract for provenance, retention, and source verification.
   Dependency lookup uses the installed layout. Repository discovery continues to
   exclude `node_modules`. [Dependency contract](../visual/DEPENDENCIES_PRODUCT.md)
