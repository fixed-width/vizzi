# Source-workspace lifetime

For contributors implementing editor hosts. A source-workspace lifetime module owns
semantic sharing and its editor-launched native process. Editor selection, notifications
and presentation remain in the editor adapter. This contract complements
[captured-source handoff](HANDOFF.md) and [originating-editor return](ORIGIN_EDITOR.md).

## Ordering and ownership

- Register an operation before asynchronous folder, source or revision selection.
  Stop Workspace invalidates every earlier operation immediately, including unresolved
  selection and configuration reads. Obsolete results and errors remain silent.
- A new operation after Stop waits for owned resources to finish closing, then may
  start a fresh workspace. It cannot join a cancelled startup.
- The latest selected source workspace replaces a pending launch for another root.
  A late earlier resolution cannot retarget the workspace. A → B → A finishes at A;
  a retired A launch cannot be adopted by the later A request.
- Same-root operations first join the captured native startup, including its initial
  sharing acquisition and original failure. They refresh sharing only after that
  startup succeeds; a failed shared attempt cannot silently launch another process.
  Their preparation and presentation remain independent, so ordinary Open followed
  by Compare still reaches Review. Existing source-action supersession remains separate.
- Caller cancellation cancels its operation, preserving shared native capture.
  Root replacement, Stop and native exit invalidate affected operations before further
  preparation or presentation can take effect.

## Separate lifetimes

Stop Sharing closes only semantic sharing, including pending creation. It does not
cancel native startup, close source views or break originating-editor return. An earlier
Open cannot restore sharing after Stop Sharing; a subsequent explicit Open can. A native
launch uses the stable root-derived semantic connection path even if no listener is
available; semantic analysis then reports unavailable rather than starting a fallback.
Advanced sharing can select another root without replacing the native process.

Closing the Vizzi tab retains the native process and origin listener. A project switch,
Stop Workspace or extension deactivation closes them. An unexpected current native
exit retires its tab and origin listener but keeps sharing. Restart requires a new
explicit action. An old process exit cannot retire a replacement process or tab.
Browser-opening failure retains the native process and reports its address. Tab-disposal
failure during Stop cannot prevent native or sharing cleanup; Stop reports that editor
error after resources close. A failed project switch has already scheduled old-native
cleanup before reporting, and a background exit records tab errors through the editor adapter.

## Module interface and adapters

`source-workspace.cjs` exposes `use`, `stop` and terminal `dispose`. `use` registers the
operation, selects and resolves its project, acquires resources, prepares native work,
then presents a current result. It returns the result, `undefined` for cancellation or
supersession, or the original current failure. The notification owner remains the command.

`workspace-runtime.cjs` supplies the VS Code resource adapter: canonical project resolution,
provider/semantic-listener construction and native launch settings. `launcher.cjs` retains
child and originating-listener cleanup. `workspace-view.cjs` retains tab creation, readiness
and message delivery. No universal editor runtime or native broker is introduced.

Preparation and presentation callbacks receive a scoped cancellation signal. They must
pass it to asynchronous native operations and editor delivery, checking it before effects
following waits. The lifetime module checks before and after each callback and drops
obsolete errors/results; it cannot retract an already dispatched editor or browser effect.

Every resource-producing factory must either clean partial acquisition on failure or
return an owned handle with an idempotent disposer. The module retains ownership until
the factory settles, including after cancellation. A late handle is closed once before
replacement proceeds. Stop invalidates immediately but completes only after cleanup;
a permanently noncooperative factory cannot be made safe by abandoning its promise.

## Verification

Exercise the module's interface with deterministic deferred selection, factories and
preparation: Stop with queued browser action, fresh Open during cleanup, A → B → A,
late resolution/failure, Stop Sharing at each startup phase, shared startup errors,
stale exits, source-intent cancellation and late resource cleanup. Keep focused command
adapter checks and real child/socket/installed-editor checks. Move ownership assertions
out of import-reconstructed command fixtures as the module takes responsibility.
