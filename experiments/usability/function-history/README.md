# Run the function-history reading trial

Facilitator how-to for [issue #6](https://github.com/fixed-width/vizzi/issues/6).
Use the existing [human-trial protocol](../PROTOCOL.md) for assistance, exposure and
interpretation. This task evaluates the delivered history, adjacent comparison and
same-file captured-call slice. [Preparation status](PREPARATION.md) is separate from
participant outcomes.
The [current findings](FINDINGS.md) retain participant reports and resulting decisions.

## Prepare the session

Use a complete local lpc-rs repository containing these commits:

| Input | Full commit |
| --- | --- |
| Workspace HEAD / review target | `457446c870ca3144c315ebcae9c78b7bc2b76514` |
| Review base, target's first parent | `2a6a23ad1be7e7892ce527fa77aab8eea8203353` |

The task starts at `src/interpreter/task/mod.rs::Task::run_entry`. Supplying the
declaration is intentional: this trial studies reading and retaining a call trail,
rather than discovering an entry point. Both questions use the real committed change;
there is no injected defect or predetermined requirement to reject the patch.

Build Vizzi itself, then create a fresh disposable clone. These commands neither build
lpc-rs nor change its original worktree, refs or index. A shared clone needs the original
Git object store to remain available throughout the session.

```sh
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --locked --offline -p vizzi
TRIAL_DIR=/tmp/vizzi-history-trial-u02
LPC_SOURCE=/path/to/lpc-rs
mkdir "$TRIAL_DIR"
mkdir -p "$TRIAL_DIR/participant" "$TRIAL_DIR/facilitator"
git -c core.hooksPath=/dev/null clone --shared --no-checkout \
  "$LPC_SOURCE" "$TRIAL_DIR/participant/source"
git -C "$TRIAL_DIR/participant/source" -c core.hooksPath=/dev/null checkout --detach \
  457446c870ca3144c315ebcae9c78b7bc2b76514
cp experiments/usability/function-history/participant/*.md "$TRIAL_DIR/participant/"
cp experiments/usability/function-history/facilitator/session.md \
  "$TRIAL_DIR/facilitator/session.md"
```

Save original HEAD, refs, index and status digests before/after preparation. Verify the
pin, complete history and selected source objects locally. Freeze the task, control
reference and [scoring reference](facilitator/scoring.md), including their hashes,
before starting. Do not alter the rubric after receiving an answer. Keep facilitator
files, preparation console and guided readiness screenshots out of the participant
handoff. Only `participant/` is supplied to the reader.

## Launch the VS Code condition

Use the current source extension with the current executable in an isolated profile.
This avoids depending on the version of the participant's installed preview. Save
`editor-profile/User/settings.json` beneath the trial directory with these settings,
replacing the executable path with the absolute path to the build:

```json
{
  "vizzi.executablePath": "/absolute/path/to/vizzi/target/release/vizzi",
  "rust-analyzer.checkOnSave": false,
  "rust-analyzer.cargo.buildScripts.enable": false,
  "rust-analyzer.procMacro.enable": false,
  "extensions.autoUpdate": false,
  "extensions.autoCheckUpdates": false,
  "update.mode": "none"
}
```

The Rust settings prevent automatic checks, build scripts and procedural macros in
the inspected repository. They are part of this condition, not a claim about the
participant's normal editor configuration. Do not invoke project builds during setup.
Save `participant/source/vizzi.toml` with:

```toml
root = "."
workers = 1
port = 0
cache = "../../editor-cache"
```

Record this added launch file separately from the pinned project bytes. From the Vizzi
checkout, launch:

```sh
code --new-window --user-data-dir "$TRIAL_DIR/editor-profile" \
  --extensionDevelopmentPath "$PWD/editors/vscode" \
  "$TRIAL_DIR/participant/source" "$TRIAL_DIR/participant/task.md"
```

The participant opens **Vizzi: Open Workspace** in that window. Record the actual
extension, executable hash, VS Code version, OS, display size and orientation. If a
trusted-workspace prompt appears, resolve it before timing. Keep startup failures and
facilitator assistance in the record. Stop through **Vizzi: Stop Workspace** when done.

Readiness automation must use a separate native process, cache and browser storage.
It can check source bytes, history stepping, comparison and captured-helper opening.
Include the declaration spelling supplied in the participant brief in search checks;
classify it as guided, source-informed evidence. Start the human condition with an empty
Vizzi task. Do not pre-populate the expected call trail or show the automated route.

## Facilitate and report

Record repository, transactional-system and tool familiarity before the attempt, plus
recognition of this change. Offer about 20 minutes and allow pauses or stopping. Give
the neutral [task](participant/task.md) and [controls](participant/controls.md). The
participant chooses their reading path and may use VS Code; record each editor switch
and the question it answered. There is no editor-only baseline in this condition.

Collect the learning answer before the review answer. Review is an exploratory second
task with learning exposure; do not pool it with an independent review attempt. After
both answers, probe any unobserved controls: previous/next history, Before/After,
following a captured helper, returning to an earlier bubble and Return to live. Record
which actions were spontaneous, prompted, assisted or unattempted. A missing operation
is missing evidence, not a successful interaction.

Use the [private session sheet](facilitator/session.md) for the actual path, interruptions,
assistance and concrete incidents. Record unsupported cross-file, arbitrary-revision or
lineage needs when they arise; they are not prerequisites to this trial. Assess source
evidence separately from clicks, elapsed time and confidence.

Keep raw participant notes outside Git by default. After the session, prepare a
de-identified findings summary for explicit review before publishing. Link each product
decision to an observed incident, and create separate issues for reproducible actionable
defects with their reproduction evidence. Missing participant observations leave #6 incomplete. Neither
successful readiness checks nor a single maintainer session establish a comprehension
or productivity improvement over an editor.
