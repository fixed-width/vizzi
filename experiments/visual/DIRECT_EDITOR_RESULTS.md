# Direct fragment editor handoff

For maintainers verifying the removal of the Keep prerequisite, 2026-11-09.
**Editor** now opens a current fragment at its original declaration line. **Keep** remains
independent; opening does not create evidence or change the investigation revision.

The native `POST /api/editor/open` request carries snapshot/file/version/line bindings.
It rejects obsolete observations, wrong document versions and invalid lines, then uses
the same no-follow working-file read and full content-identity check as saved evidence.
The saved-evidence route remains available. Both routes share the configured editor
adapter and the single editor-operation allowance. No new dependencies or unsafe code
were introduced.

Formatting, workspace tests/doctests and Clippy with warnings denied pass: **592 tests
passed, 3 opt-in real-provider tests ignored** by the normal workspace command. The full
browser suite also passes. Tests first reproduced the missing native interface and the
rendered disabled button, then verified:

- Opening an unkept fragment without changing evidence, revision or blocked persistence.
- Opening while the browser's investigation-state request fails; Keep remains independent.
- Snapshot/file/version/line validation, exact line boundaries, unpublished indexing,
  same-length working-file changes before reindexing, missing files, matching-byte symlinks,
  and editor refusal.
- Strict HTTP shape/body bounds and stale-source rejection, with no client-supplied path,
  command or executable accepted.
- Existing saved handoff, literal VS Code arguments, large-file verification and source
  freshness checks through the shared verifier.

Every automated editor handoff used a recording or refusing adapter/executable. No desktop
VS Code window was opened. As before, CLI acknowledgement does not confirm an existing
editor buffer's contents; unsaved buffers and filesystem changes after verification require
richer editor integration.

Mutation testing used a passing unmodified baseline, one mutant/compiler task, serial tests
and nice 10. The selected full handoff/verifier methods and shared HTTP helper produced
**33 mutations: 28 caught, 4 unviable, 1 equivalent survivor, 0 timeouts**. The tool also
selected removal of `outline_count` in evidence pinning. That survivor changes an unused
outline projection; pinning consumes only the source lines, so evidence and handoff results
are unchanged. Timing effects were not tested. No mutants were excluded. DOM-controller
behaviour has browser coverage rather than a complete mutation scope.

[Compressed logs, mutation outcomes and checksums](results/direct-editor/checksums.json)
record the checks. The [usage guide](../../docs/how-to/follow-a-call-trail.md) and
[handoff reference](../../docs/companion/API.md) describe the user workflow and protocol.
