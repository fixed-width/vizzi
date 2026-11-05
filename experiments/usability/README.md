# Run a learning and review trial

This how-to is for a facilitator preparing a local session. Use the
[protocol](PROTOCOL.md) to classify attempts and interpret observations, and keep the
[scoring reference](facilitator/scoring.md) separate from participant material.
The first maintainer trial produced [interaction findings](FINDINGS.md), with
[the implementation and validation response](FEEDBACK_RESULTS.md). Retest those
interactions with the updated viewer; unfamiliar-user acceptance stays open.

## Prepare disposable inputs

Run from the Vizzi checkout with Python 3.12+, Git, a built Vizzi binary and a local
lpc-rs repository containing the task pin. Choose a new trial directory. The commands
archive/materialise source; they do not build or run lpc-rs.

```sh
TRIAL_DIR=/tmp/vizzi-trial-u01
LPC_SOURCE=/path/to/lpc-rs
mkdir "$TRIAL_DIR"
mkdir -p "$TRIAL_DIR/learning/workspace" "$TRIAL_DIR/participant" "$TRIAL_DIR/facilitator"
git -C "$LPC_SOURCE" -c core.fsmonitor=false -c core.hooksPath=/dev/null \
  archive --format=tar --output "$TRIAL_DIR/facilitator/baseline.tar" \
  1a4f5b65dd2a269298748df2dfd68092a169a8d9
python3 - "$TRIAL_DIR/facilitator/baseline.tar" "$TRIAL_DIR/learning/workspace" <<'PY'
import sys, tarfile
with tarfile.open(sys.argv[1]) as archive:
    archive.extractall(sys.argv[2], filter="data")
PY
```

If creating the trial directory fails because it exists, choose another directory
before continuing.

Choose one review case privately. For the later unfamiliar-user pilot, record
allocation before the session and cover both frozen cases with different reviewers.
For a maintainer trial, either case is suitable. Do not show the case label, manifest,
oracle, preparation console or scripted route to a participant before their answer.

```sh
python3 -B experiments/review-workflow/materialize.py \
  --source "$LPC_SOURCE" --variant variant-a --history --output "$TRIAL_DIR/review"
python3 -B experiments/review-workflow/verify.py --fixture "$TRIAL_DIR/review"
```

The verifier's output includes the case label; save it with facilitator records.
For a maintainer revision trial, `--history` includes committed objects from the local
source repository, with one Git packing thread and no network fetch. HEAD stays at the
task pin and the frozen working/index patches stay the same. It supports ancestor
expressions and available short hashes; other branch names are not copied. Omitting the
flag keeps the minimal single-commit fixture used in existing scripted review cohorts.
The learning archive is intentionally a source copy, not the patched review repository.
Keep the review manifest outside the source workspace. If sharing files with another
machine, share only the assigned task/controls/notes and its source workspace, using
neutral directory names. Supply a Vizzi build for that machine separately.

## Give the operator the task

```sh
cp experiments/usability/participant/controls.md "$TRIAL_DIR/participant/controls.md"
cp experiments/usability/participant/notes.md "$TRIAL_DIR/participant/notes.md"
cp experiments/usability/participant/learning.md "$TRIAL_DIR/participant/learning.md"
cp experiments/review-workflow/fixtures/task.md "$TRIAL_DIR/participant/review.md"
```

Those copies prepare a maintainer trial of both tasks. For an unfamiliar primary
attempt, give only the assigned learning or review brief and the common controls/notes;
leave the other task with the facilitator. Start with the neutral brief, not the
guided route. Answers can be written in the notes file or reported in conversation.
Record actual task order and previous exposure.

## Start the local workspaces

If needed, build Vizzi itself with `CARGO_BUILD_JOBS=1 cargo build --release --offline
-p vizzi`. Run each host in its own terminal, with caches outside inspected source:

```sh
target/release/vizzi --root "$TRIAL_DIR/learning/workspace" \
  --cache "$TRIAL_DIR/learning/cache" \
  --investigation-file "$TRIAL_DIR/learning/investigation.json" --port 0 --workers 1
```

```sh
target/release/vizzi --root "$TRIAL_DIR/review/workspace" \
  --cache "$TRIAL_DIR/review/cache" --port 0 --workers 1
```

Use the first printed URL for the editor-companion learning trial and append `/review`
to the second printed URL. The companion starts empty; follow the
[condition/order procedure](../../docs/companion/TRIAL.md) and keep editor-only observations
separate. For a repeat of the earlier source-browser trial, append `/explore` instead
and label that condition explicitly.
The learning page's mode links stay on its learning archive; use the supplied review
URL for the patched review task. In review, use the task's full baseline pin and subtree
`.`. Keep C-family capture disabled for this corpus. Source and caches are disposable;
original checkouts remain untouched. Ctrl-C stops each foreground host.

For one unfamiliar primary task, prepare/start only its workspace. A maintainer can do
both sequentially, with each attempt labelled. Do not refresh or replace their source
silently during the session. Optional interaction probes follow the recorded answer.

## Keep preparation and observations separate

Record the Vizzi commit/binary hash, source pin, private case assignment, fixture
verification and original-checkout status digest with facilitator records. Check assets
and passive status for readiness; such checks need not navigate the task or provide an
answer. Keep session notes/recordings outside Git by default.

After the attempt, retain the answer/evidence, assistance and concrete interaction
incidents. Apply the rubric afterwards. A preparation report has no participant answers
or acceptance verdict. The [preparation record](PREPARATION.md) documents the ready
maintainer session and its limits.
