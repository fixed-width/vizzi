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
   named class declarations and their ordinary identifier-named methods, and
   TypeScript interfaces, enums and type aliases. Function ranges retain the
   exact saved text, including a variable's name and initializer when it supplies
   the function name. Nested names are qualified by the enclosing declarations in
   the outline.
3. **Add bubble** can find those functions. **Calls** records explicit call and
   constructor expressions and the written identifier on simple or dotted
   callees, including through parentheses and TypeScript's type-only wrappers.
   Computed callees stay unnamed, and JSX tags alone create no call
   evidence. Calls use the existing same-name candidate behavior, including
   ambiguity and missing destinations. For overloads, the outline includes the
   implementation body when one is present.
4. Edits use the existing captured-source refresh. The source remains browsable
   when parsing recovers or the input encoding makes syntax unavailable. Raw
   navigation works offline and requires only the captured source.

Project-aware import and type resolution, package labels, and TypeScript language
server integration are later slices.
