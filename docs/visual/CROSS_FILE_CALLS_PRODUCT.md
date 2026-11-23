# Follow a historical call into another file

A reader looking at a function from a Git commit can follow a written call into another
file at that same commit. The first release uses the same controls and evidence rules for
every language with written-call and declaration analysis. [Issue #7](https://github.com/fixed-width/vizzi/issues/7)

## Behaviour

1. When an inline call or Calls entry has a captured written name, the reader can look
   for its declaration in another file from the caller's commit. The reader browses
   paths in that commit or enters a repository-relative path directly. This works in
   Rust, Python, Ruby, C, C++, Java, JavaScript, TypeScript and TSX wherever the configured
   source policy and syntax analysis supply the required facts. Existing same-file
   actions remain available. Escape or Close returns keyboard focus to the inline call
   or the bubble's Calls button. [Language support](../reference/languages.md) ·
   [Captured-file call behaviour](CAPTURED_CALLS_PRODUCT.md)
2. Show matching declarations in the selected file with their qualified names, paths
   and exact commit. The reader confirms a cross-file name candidate, including a sole
   match. Ambiguous matches remain choices. A call without a captured name, a file with
   unavailable analysis or a search with no matches has an explicit status. Bounds on
   the file list or declaration search are visible.
3. The callee opens beside its caller. The caller retains its rectangle and reading
   position. The callee inherits task membership, and the workspace keeps its zoom.
   History bubbles and committed comparison sides can follow these calls. A comparison
   side showing a saved working-tree capture continues to support its captured file.
4. Further calls and revisits stay at the same commit. Cycles reuse an existing bubble
   in the same capture context. Another commit or live bubble remains independently
   readable even when its file has identical text. Links retain the revision and
   evidence with which they were created.
5. The callee and its link enter the saved task after source retention is confirmed.
   Task reload restores source and call links while the native capture remains available.
   A missing object, source limit, expired owner, canceled request or superseded result
   leaves the reader's current source and call trail intact. Keep the task's existing
   bubble and link limits. Return to live resolves the selected function using current
   saved source and marks earlier call links as historical.
6. Local Git objects at the indicated commit supply both endpoints and their evidence.
   A cross-file name match records a candidate relationship. Import aliases, re-exports,
   generated code, external dependencies and receiver dispatch require evidence beyond
   this name lookup. Existing source-local bindings keep their evidence.
