# Fixed-source model comparison review

For evaluators. Gold facts are not model inputs. Dispatch oracle erratum is applied explicitly. Agent grading is not independent human acceptance.

## qwen3.5-2b-q4 / decomp.add-efun.full/answer_first

how do i add a new efun?

Native checks: True.

- missing: Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- supported: Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- missing: Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "answer": "To add a new efun, you must define its prototype using the `efuns!` macro in `src/interpreter/efun/mod.rs`. The macro requires the efun name, return type, arity (number of arguments), and argument types. For example, to add a function that returns the absolute value of a string, you would define it as `abs_string` with a return type of `LpcType::Mixed`, arity of 1, and an argument of type `LpcType::String`. The macro handles the dispatch logic, so you only need to specify the prototype definition.",
  "status": "answered",
  "citations": [
    3,
    5
  ]
}
````

- [3] src/interpreter/efun/mod.rs:199-331
- [5] src/interpreter/efun/mod.rs:759-763

Finds the efuns! signature row mechanism but misses handler and tests/documentation. The hypothetical string-absolute-value example is not counted as a repository fact or a complete implementation. Correct answered status alone does not make the procedure correct.

Unsupported: Says only prototype definition is needed, omitting the native handler/module required by generated dispatch.; Overstates optional arity/argument-type fields as mandatory.

## qwen3.5-2b-q4 / decomp.add-efun.handler/answer_first

What Rust context/interface does a native efun handler use to read an argument and return a result?

Native checks: True.

- supported: An EfunContext supplies arg(index) and return_efun_result(value); the Rust handler returns Result<()>.

````json
{
  "answer": "A native efun handler uses the `EfunContext` interface, which provides methods like `arg` to read arguments and `return_efun_result` to return results.",
  "status": "answered",
  "citations": [
    1,
    5
  ]
}
````

- [1] src/interpreter/efun/abs.rs:1-69
- [5] src/interpreter/efun/mod.rs:759-763

Correct EfunContext, arg and return_efun_result. Label 1 supports all material claims; extra label 5 is unnecessary but does not replace that supporting reference.

## qwen3.5-2b-q4 / decomp.add-efun.registration/answer_first

Where is a native efun signature registered, and does the same registration also generate its runtime dispatch?

Native checks: True.

- supported: Edit an efuns! table row in src/interpreter/efun/mod.rs, not a separate hand-maintained prototype map.
- supported: The macro generates both EFUN_PROTOTYPES entries and runtime dispatch from those rows.

````json
{
  "answer": "The native efun signature for `abs` is registered in `src/interpreter/efun/mod.rs` at line 759 under the macro `efuns!`. It is defined as `abs => { returns: LpcType::Mixed(false), arity: 1, args: [LpcType::Int(false) | LpcType::Float(false)], }`. The same registration generates its runtime dispatch via the `@dispatch` arm within the `efuns!` macro, which maps the efun name to the corresponding function implementation (`abs`), allowing the interpreter to route calls to the correct native function.",
  "status": "answered",
  "citations": [
    5,
    6
  ]
}
````

- [5] src/interpreter/efun/mod.rs:759-763
- [6] doc/efun/abs.md:1-11

Correct efuns! row and generated dispatch mechanism, unlike its status-first partner. However, citations 5 and 6 cover the example row and abs Markdown; they do not establish the requested dispatch/prototype-generation relationship in macro source 3. Factual content is credited separately from complete citation support.

## qwen3.5-2b-q4 / decomp.transaction-creation.full/answer_first

where are new transactions created?

Native checks: True.

- supported: The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- contradicted: A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "answer": "New transactions are created in the `Transaction::new` method, which is called from `start_txn` within the `open_attempt` flow. Specifically, `start_txn` sends a `CommitProtocol::Start` message to the committer, which then creates a `LiveSnapshot` and passes it to `Transaction::new` (as seen in `committer.rs:301`). This `Transaction` is then wrapped in a `TxnHandle` and used to begin a new attempt, as shown in `command_task.rs:108`. The creation of the transaction object itself occurs at line 127 of `src/interpreter/stm/mod.rs` where `Transaction::new(snapshot)` is invoked.",
  "status": "answered",
  "citations": [
    1,
    2,
    3,
    4,
    5
  ]
}
````

- [1] src/interpreter/stm/mod.rs:127-143
- [2] src/interpreter/task/mod.rs:377-442
- [3] src/interpreter/stm/retry.rs:477-494
- [4] src/interpreter/stm/committer.rs:289-301
- [5] src/command/command_task.rs:84-108

Correctly locates the constructor; accept location without requiring unasked initializer-field spelling. It names open_attempt and improves the Start sender direction, but wrongly attributes transaction construction to the helper/committer instead of the owning caller.

Unsupported: Claims Transaction::new is called from start_txn and that the committer passes its LiveSnapshot directly into that constructor.

## qwen3.5-2b-q4 / decomp.transaction-creation.task-entry/answer_first

Which method on Task creates a fresh transaction for a non-joining attempt?

Native checks: True.

- supported: Task::open_attempt obtains a LiveSnapshot and assigns TxnHandle::new(Transaction::new(...)) to its context.

````json
{
  "answer": "The method `open_attempt` on the `Task` struct creates a fresh transaction for a non-joining attempt. It calls `start_txn` to begin the transaction, which in turn invokes a `request` to send a `CommitProtocol::Start` message to the committer. This process generates a `LiveSnapshot` that is then passed to `Transaction::new` to establish the transaction context.",
  "status": "answered",
  "citations": [
    2
  ]
}
````

- [2] src/interpreter/task/mod.rs:377-442

Correct Task::open_attempt and the sequence of obtaining a LiveSnapshot then creating the Transaction. Source 2 directly supports the requested method/creation step; the ancillary Start helper description is also established elsewhere in the supplied context.

## qwen3.5-2b-q4 / decomp.transaction-creation.start-message/answer_first

In start_txn, which side sends CommitProtocol::Start, and what comes back from the committer?

Native checks: True.

- supported: The caller/client helper start_txn sends Start through the committer channel.
- supported: Committer::process handles Start and returns a LiveSnapshot of its current world.

````json
{
  "answer": "In `start_txn`, the `request` function sends `CommitProtocol::Start` via a `flume::Sender<CommitProtocol>`. The committer responds by sending back a `LiveSnapshot` containing the current snapshot and the transaction handle.",
  "status": "answered",
  "citations": [
    3,
    4
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-494
- [4] src/interpreter/stm/committer.rs:289-301

Correct sender and typed LiveSnapshot reply from sources 3/4. Transaction handle is ambiguous shorthand for the reply/release handle; it does not explicitly assert a TxnHandle field. Do not penalise an unasked internal-layout interpretation.

## qwen3.5-2b-q4 / decomp.commit-checks.full/answer_first

show me the codepath that transaction commits take, with all the checks they go through

Native checks: True.

- contradicted: The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- supported: Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- supported: A base version older than retained history is rejected as HistoryUnavailable.
- supported: Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- missing: Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- missing: Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- missing: Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- missing: The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "answer": "The transaction commit codepath in `src/interpreter/stm/committer.rs` proceeds as follows: First, it validates the changeset against the current snapshot version, checking for future versions, history unavailability, and read invalidation conflicts. If the changeset folds successfully, it publishes the new version, updates the write history, and sends the commit result via the `Committer::dispatch` handler. The `commit` function specifically handles the `Committer::dispatch` case, which calls `self.commit(changeset)` to execute the actual commit logic. This involves cloning the changeset, sending it to the committer, and handling the `Committer::commit` method which performs the core commit operation and releases the base version if `releases_base` is true.",
  "status": "answered",
  "citations": [
    3,
    4,
    5
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523
- [4] src/interpreter/stm/committer.rs:302-321
- [5] src/interpreter/stm/committer.rs:611-709

dispatch exists; remove the erroneous invention finding. The response still conflates the commit validator with protocol handling and release ownership, so the broad task remains incorrect.

Unsupported: Conflates the dispatch protocol handler with commit validation and assigns base-pin release to the wrong method.

## qwen3.5-2b-q4 / decomp.commit-checks.future-base/answer_first

What happens if a changeset base version is newer than the committer current version?

Native checks: True.

- supported: Committer::commit rejects it with Conflict::FutureVersion before publication.

````json
{
  "answer": "If a changeset base version is newer than the committer's current version, the system returns an error `Conflict::FutureVersion` with details about the base and current versions. This occurs because the committer cannot commit a change that predates its current state.",
  "status": "answered",
  "citations": [
    3
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523

Correctly states FutureVersion with base/current details, but adds the wrong rationale and cites transport helper 3 instead of validation block 5.

Unsupported: Explains a newer-than-current base as a change that predates the current state, reversing the version relationship.

## qwen3.5-2b-q4 / decomp.commit-checks.old-base/answer_first

What happens if a changeset base version is older than the retained write history?

Native checks: True.

- supported: It is rejected as Conflict::HistoryUnavailable because the required validation history is unavailable.

````json
{
  "answer": "If a changeset base version is older than the retained write history, the committer returns an error with `Conflict::HistoryUnavailable`. This occurs because the system lacks sufficient history to resolve the conflict rule, as the required older versions are no longer retained.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Direct, correct HistoryUnavailable rejection and missing-history rationale, with the actual validation source 5.

## qwen3.5-2b-q4 / decomp.commit-checks.read-conflict/answer_first

Which committed writes are checked for invalidated reads, and what happens when one is found?

Native checks: True.

- missing: Write-history versions strictly after the base through the current version are scanned; the base itself is skipped.
- supported: A conflicting tracked read rejects the changeset as ReadInvalidated before publication.

````json
{
  "answer": "The code checks for invalidated reads by iterating through the `write_history` range from the changeset version to the current version. If a conflicting read is found, the function returns an error with `Conflict::ReadInvalidated`, which includes details about the conflicting read cell, its base version, the current version, the writer, and the time of the write. This allows the system to track which specific data was invalidated.",
  "status": "answered",
  "citations": [
    4
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321

Same missing base-skip detail and wrong supporting block as status-first. Interpret time of write as logical written_at version, not an invented wall-clock timestamp.

## qwen3.5-2b-q4 / decomp.commit-checks.merge-check/answer_first

How are merge operations checked before publication, and what happens on a mismatch?

Native checks: True.

- contradicted: fold_merges uses the currently committed values.
- supported: A merge/type mismatch rejects the entire changeset as MergeMismatch before any changes apply.

````json
{
  "answer": "Merge operations are checked by iterating through the `write_history` range from the changeset version up to the current version. If a conflicting read is found, the transaction returns an error with `Conflict::ReadInvalidated`. If the merge fold fails (indicating a type mismatch), the transaction returns an error with `Conflict::MergeMismatch`. In both cases, the `stats.conflict()` counter is incremented.",
  "status": "answered",
  "citations": [
    4
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321

Correct mismatch error, but the operation and citation are wrong. No improvement from field order on this pair.

Unsupported: Conflates read-invalidation history checks with merge-fold validation against committed values.

## qwen3.5-2b-q4 / decomp.commit-checks.commit-receiver/answer_first

Which Committer method receives CommitProtocol::Commit, and which method does it call to validate the changeset?

Native checks: True.

- supported: Committer::dispatch directly handles CommitProtocol::Commit, via the process wrapper, and invokes self.commit(changeset) for validation.

````json
{
  "answer": "The `Committer::dispatch` method receives `CommitProtocol::Commit`. It calls the `commit` method to validate the changeset, which checks version ordering, history availability, and conflict rules.",
  "status": "answered",
  "citations": [
    4,
    5
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321
- [5] src/interpreter/stm/committer.rs:611-709

Correct dispatch receiver and commit validator, supported by actual dispatch scope/block 4 and validator 5. Earlier rejection was an oracle error: process wraps dispatch. Corrected under the explicit dispatch erratum; raw response is unchanged.

## qwen3.5-2b-q4 / decomp.commit-checks.read-only/answer_first

Does a conflict-free read-only commit create a new version or a write-history entry?

Native checks: True.

- contradicted: No: publish records a read-only commit and returns without creating a version or history entry; validation has already run.

````json
{
  "answer": "A conflict-free read-only commit creates a new version but does not create a write-history entry. This is because the `publish` method in `src/interpreter/stm/committer.rs` explicitly checks if `written_vars` is empty. If it is, the code logs `read_only_commit()` and returns early, skipping the insertion of a `CommittedWrite` entry into `self.write_history`. The version is still created via `new_version = Version::new()` and `self.snapshot.apply`, but the history is not updated.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Cites the correct block but reverses its early-return effect. Ordering does not correct this control-flow error.

Unsupported: Claims a new version/snapshot is created for an empty write set even though publish returns before those statements.

## qwen3.5-2b-q4 / decomp.commit-checks.effects/answer_first

When does the attempt runner deliver physical effects, and what does it do after a rejected commit?

Native checks: True.

- supported: Effects are delivered only after an Ok commit result.
- contradicted: A rejected commit records the conflict and retries a fresh attempt with admission/backoff subject to the evaluation deadline.

````json
{
  "answer": "The attempt runner delivers physical effects after the commit phase completes successfully, as seen in `src/interpreter/stm/retry.rs:393` where `body.deliver(effects).await` is called following `body.commit_phase(&self.tx, live).await`. After a rejected commit, the runner enters a retry loop: it increments the attempt counter, checks if the deadline has passed, and if the commit failed due to a conflict (e.g., `Conflict::ReadInvalidated`), it stores the conflicting cell in `retry_cell` to be used in the next attempt (see `src/interpreter/stm/retry.rs:399-400`). If no retry cell is available, it applies the backoff strategy before breaking the loop.",
  "status": "answered",
  "citations": [
    1,
    3
  ]
}
````

- [1] src/interpreter/stm/retry.rs:277-416
- [3] src/interpreter/stm/retry.rs:477-523

Correct accepted-commit delivery and some conflict/admission details. The claimed loop exit and inline locations are wrong despite a supporting full-runner reference.

Unsupported: Says backoff is applied before breaking the attempt loop, although the loop continues after a conflict.; Invents exact retry.rs line positions for delivery/retry-cell operations.

## qwen3.5-2b-q4 / decomp.commit-checks.base-pin/answer_first

When is the base-version pin released during commit processing, and when may old history be evicted?

Native checks: True.

- supported: When releases_base is true, the base pin is released after commit validation, including rejection.
- supported: Old history is evicted only when live_versions is empty.

````json
{
  "answer": "The base-version pin is released during the `commit` phase of the `Committer::dispatch` handler, specifically when the `releases_base` flag is set in the `CommitProtocol::Commit` message. This occurs after the commit logic validates the changeset and folds merges, ensuring the base version's pin is released. Old history may be evicted if the `live_versions` list becomes empty, which triggers the `evict_old_versions()` method in the committer. This eviction check happens immediately after the commit phase completes successfully.",
  "status": "answered",
  "citations": [
    3,
    5
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523
- [5] src/interpreter/stm/committer.rs:611-709

Correct release flag/phase and live_versions-empty eviction. dispatch is the real handler, not an invented method. Citation support still fails because labels 3/5 omit actual release/eviction block 4. Corrected under the dispatch erratum.

## qwen3.5-4b-q4 / decomp.add-efun.full/answer_first

how do i add a new efun?

Native checks: True.

- supported: Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- supported: Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- missing: Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "answer": "To add a new efun in the `lpc-rs` repository, you must perform three steps:\n\n1. **Implement the function**: Create a new module (e.g., `src/interpreter/efun/<name>.rs`) containing the implementation. The function must accept an `&mut EfunContext<'_, N>` and return a `Result<()>`. It should use `context.arg()` to read arguments and `context.return_efun_result()` to set the return value.\n2. **Register the prototype**: Add an entry in the `efuns!` macro in `src/interpreter/efun/mod.rs`. This defines the efun's name, return type, arity, argument types, and flags. For example:\n   ```rust\n   <name> => {\n       returns: LpcType::Mixed(false),\n       arity: 1,\n       args: [LpcType::Int(false) | LpcType::Float(false)],\n   },\n   ```\n3. **Update documentation**: Create a markdown file in `doc/efun/<name>.md` describing the efun's behavior. The build script (`build.rs`) automatically generates documentation code from these files.\n\nThis process is supported by the repository's structure, where the `efuns!` macro dispatches to implementations and the `EFUN_PROTOTYPES` map indexes them for assembly.",
  "status": "answered",
  "citations": [
    1,
    2,
    3,
    5,
    6
  ]
}
````

- [1] src/interpreter/efun/abs.rs:1-69
- [2] src/interpreter/efun/mod.rs:3-3
- [3] src/interpreter/efun/mod.rs:199-331
- [5] src/interpreter/efun/mod.rs:759-763
- [6] doc/efun/abs.md:1-11

Correct native handler/context interface, efuns! registration/dispatch and Markdown path. Treat Rust module creation plus cited declaration as sufficient for the handler/module portion. The procedure omits adding tests, so the combined tests/documentation fact and full workflow remain incomplete. Existing signature is clearly an example, not a claim that every efun needs these types.

## qwen3.5-4b-q4 / decomp.add-efun.handler/answer_first

What Rust context/interface does a native efun handler use to read an argument and return a result?

Native checks: True.

- supported: An EfunContext supplies arg(index) and return_efun_result(value); the Rust handler returns Result<()>.

````json
{
  "answer": "A native efun handler uses the `EfunContext` to read arguments and return results. Specifically, the handler receives a mutable reference to `EfunContext<'_, N>` (where `N` is the stack size). It reads arguments via the `arg(index)` method, which returns a `LpcRef`, and returns the result by calling `return_efun_result(result)`. This pattern is demonstrated in the `abs` function implementation.",
  "status": "answered",
  "citations": [
    1
  ]
}
````

- [1] src/interpreter/efun/abs.rs:1-69

Correct EfunContext, arg(index), return_efun_result and abs example, directly supported by handler source 1. LpcRef names the argument value type; no unsupported owned-versus-borrowed assertion is inferred.

## qwen3.5-4b-q4 / decomp.add-efun.registration/answer_first

Where is a native efun signature registered, and does the same registration also generate its runtime dispatch?

Native checks: True.

- supported: Edit an efuns! table row in src/interpreter/efun/mod.rs, not a separate hand-maintained prototype map.
- supported: The macro generates both EFUN_PROTOTYPES entries and runtime dispatch from those rows.

````json
{
  "answer": "The native efun signature is registered in the `efuns!` macro within `src/interpreter/efun/mod.rs`, specifically in the `EFUN_PROTOTYPES` section (e.g., line 759-763 for `abs`). This registration defines the return type, arity, and argument types. The same macro block also generates the runtime dispatch logic via the `call_efun` function, which matches the efun name to its corresponding implementation module (e.g., `src/interpreter/efun/abs.rs`).",
  "status": "answered",
  "citations": [
    3,
    5
  ]
}
````

- [3] src/interpreter/efun/mod.rs:199-331
- [5] src/interpreter/efun/mod.rs:759-763

Correct efuns! source-row registration and generated call_efun dispatch, supported by macro definition 3 and example 5. Interpret EFUN_PROTOTYPES section as the generated prototype portion of this macro, not an instruction to maintain a separate map. Matching the efun name describes generated enum variants, not an asserted string-map lookup.

## qwen3.5-4b-q4 / decomp.transaction-creation.full/answer_first

where are new transactions created?

Native checks: True.

- supported: The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- missing: A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "answer": "New transactions are created in two primary locations within the repository code. First, the `Transaction` struct is instantiated via its `new` method in `src/interpreter/stm/mod.rs` when a `LiveSnapshot` is cloned to create a `TxnHandle` (see excerpt [1]). Second, the `LiveSnapshot` itself, which wraps the data for the new transaction, is created in `src/interpreter/stm/committer.rs` when the committer dispatches a `CommitProtocol::Start` request (see excerpt [4]). This process is triggered by tasks like `CommandTask` (excerpt [5]) or `Task` (excerpt [2]) calling `start_txn`, which ultimately invokes the creation of the snapshot and subsequent transaction handle.",
  "status": "answered",
  "citations": [
    1,
    4
  ]
}
````

- [1] src/interpreter/stm/mod.rs:127-143
- [4] src/interpreter/stm/committer.rs:289-301

Correct constructor location and high-level task/Start/current-world-snapshot creation sequence. Interpret cloning a LiveSnapshot as cloning its inner snapshot data, not an explicit call to LiveSnapshot::clone. It names Task and command callers but not the required Task::open_attempt entry method; useful partial creation survey under the unchanged two-part rubric. Inline source [2]/[5] mentions are retained, while structured citations support the constructor and Start handling.

## qwen3.5-4b-q4 / decomp.transaction-creation.task-entry/answer_first

Which method on Task creates a fresh transaction for a non-joining attempt?

Native checks: True.

- supported: Task::open_attempt obtains a LiveSnapshot and assigns TxnHandle::new(Transaction::new(...)) to its context.

````json
{
  "answer": "The method `open_attempt` on the `Task` struct creates a fresh transaction for a non-joining attempt. If `self.joins_parent` is false, it calls `start_txn` to create a new `LiveSnapshot`, which is then used to instantiate a new `Transaction` via `Transaction::new` within a `TxnHandle`.",
  "status": "answered",
  "citations": [
    2,
    3
  ]
}
````

- [2] src/interpreter/task/mod.rs:377-442
- [3] src/interpreter/stm/retry.rs:477-494

Correct Task::open_attempt, non-joining condition and LiveSnapshot-to-Transaction/TxnHandle sequence, supported by actual task method 2 and helper 3.

## qwen3.5-4b-q4 / decomp.transaction-creation.start-message/answer_first

In start_txn, which side sends CommitProtocol::Start, and what comes back from the committer?

Native checks: True.

- supported: The caller/client helper start_txn sends Start through the committer channel.
- supported: Committer::process handles Start and returns a LiveSnapshot of its current world.

````json
{
  "answer": "In `start_txn`, the caller (the side invoking `start_txn`) sends the `CommitProtocol::Start` message containing a reply channel. The committer receives this and responds by sending a `LiveSnapshot` object back through that reply channel.",
  "status": "answered",
  "citations": [
    3,
    4
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-494
- [4] src/interpreter/stm/committer.rs:289-301

Correct caller/client Start request with reply channel and committer LiveSnapshot reply; sources 3 and 4 directly show both sides.

## qwen3.5-4b-q4 / decomp.commit-checks.full/answer_first

show me the codepath that transaction commits take, with all the checks they go through

Native checks: True.

- supported: The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- supported: Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- supported: A base version older than retained history is rejected as HistoryUnavailable.
- supported: Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- supported: Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- missing: Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- missing: Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- missing: The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "answer": "The transaction commit path begins when an `AttemptBody` (e.g., a `Task`) calls `commit_phase`, which clones the changeset and invokes `commit_changeset` via a channel to the committer ([2], [3]). The committer receives the `CommitProtocol::Commit` message and calls its `commit` method ([4], [5]). Inside `commit`, the following checks occur:\n\n1. **Version Consistency**: It verifies that the current snapshot version is not less than the changeset's base version to prevent creating versions out of order ([5]).\n2. **History Availability**: It ensures the base version is within the retained history (`oldest_retained`), returning `Conflict::HistoryUnavailable` if not ([5]).\n3. **Conflict Detection**: It iterates through the write history between the base and current versions. If the changeset's read set overlaps with a write from a later version, it returns `Conflict::ReadInvalidated` ([5]).\n4. **Merge Validation**: It attempts to fold merges against the current snapshot. If a type mismatch is found, it returns `Conflict::MergeMismatch` ([5]).\n\nIf all checks pass, the changeset is published, a new version is created, and the result is sent back to the client via the reply channel ([4], [5]).",
  "status": "answered",
  "citations": [
    2,
    3,
    4,
    5
  ]
}
````

- [2] src/interpreter/task/mod.rs:1054-1083
- [3] src/interpreter/stm/retry.rs:477-523
- [4] src/interpreter/stm/committer.rs:302-321
- [5] src/interpreter/stm/committer.rs:611-709

Correct body/phase-to-channel-to-committer validator path, without the invented dispatch method. Accept the structural receiver/validator separation without requiring literal process spelling. All four rejection criteria are accurately described; exact FutureVersion spelling is not required for the full trace when its condition is clear. It omits the read-only no-version branch, effect/retry outcome and pin-release/eviction ordering. The unqualified new-version sentence is treated as ordinary write-path description, not completion of the missing read-only fact, consistently with the baseline grading.

## qwen3.5-4b-q4 / decomp.commit-checks.future-base/answer_first

What happens if a changeset base version is newer than the committer current version?

Native checks: True.

- supported: Committer::commit rejects it with Conflict::FutureVersion before publication.

````json
{
  "answer": "If a changeset's base version is newer than the committer's current version, the `commit` method in the committer returns an error indicating a conflict with a future version. Specifically, it returns `Err(Conflict::FutureVersion { base, current })`, where `base` is the version from the changeset and `current` is the committer's latest version. This check prevents commits from basing on versions that have not yet been created or are logically in the future relative to the committer's state.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Correct FutureVersion rejection, base/current version meanings and future-base rationale, directly supported by gate source 5.

## qwen3.5-4b-q4 / decomp.commit-checks.old-base/answer_first

What happens if a changeset base version is older than the retained write history?

Native checks: True.

- supported: It is rejected as Conflict::HistoryUnavailable because the required validation history is unavailable.

````json
{
  "answer": "If a changeset's base version is older than the oldest retained version in the committer's write history, the commit is rejected with a `Conflict::HistoryUnavailable` error. This prevents commits from using base versions that no longer exist in the system's history, ensuring consistency.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Correct HistoryUnavailable rejection and retained-history rationale, supported by source 5.

## qwen3.5-4b-q4 / decomp.commit-checks.read-conflict/answer_first

Which committed writes are checked for invalidated reads, and what happens when one is found?

Native checks: True.

- supported: Write-history versions strictly after the base through the current version are scanned; the base itself is skipped.
- supported: A conflicting tracked read rejects the changeset as ReadInvalidated before publication.

````json
{
  "answer": "Committed writes checked for invalidated reads are those recorded in the committer's `write_history` between the `changeset.base_version()` and the current snapshot version (`current_version`). Specifically, the code iterates through versions in the range `changeset_version..=current_version`, skipping the base version itself. If a `changeset.conflicting_read` is found against any `CommittedWrite` in this range, the commit is rejected with a `Conflict::ReadInvalidated` error, which includes details about the conflicting cell, the base version, the current version, the version where the write occurred, and the origin of the writer.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Correct effective write-history range, explicitly skips the base, and rejects tracked-read conflicts as ReadInvalidated with version/writer details. It cites the actual validation block 5.

## qwen3.5-4b-q4 / decomp.commit-checks.merge-check/answer_first

How are merge operations checked before publication, and what happens on a mismatch?

Native checks: True.

- supported: fold_merges uses the currently committed values.
- supported: A merge/type mismatch rejects the entire changeset as MergeMismatch before any changes apply.

````json
{
  "answer": "Merge operations are checked by calling `changeset.fold_merges` with a closure that peeks the current snapshot values for each variable ID in the changeset. If the fold operation returns an error, it indicates a type mismatch between the proposed change and the current state. On a mismatch, the committer rejects the changeset immediately by returning `Err(Conflict::MergeMismatch)` containing the conflicting cell, the base version, and the current version. This prevents the invalid merge from being published.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Correct fold_merges against current committed snapshot values and whole-changeset rejection as MergeMismatch before publication, supported by actual gate block 5.

## qwen3.5-4b-q4 / decomp.commit-checks.commit-receiver/answer_first

Which Committer method receives CommitProtocol::Commit, and which method does it call to validate the changeset?

Native checks: True.

- supported: Committer::dispatch directly handles CommitProtocol::Commit, via the process wrapper, and invokes self.commit(changeset) for validation.

````json
{
  "answer": "The `Committer::dispatch` method receives `CommitProtocol::Commit`. It then calls the `commit` method to validate the changeset.",
  "status": "answered",
  "citations": [
    4,
    5
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321
- [5] src/interpreter/stm/committer.rs:611-709

Correct direct handler is Committer::dispatch, which process wraps; it calls commit for validation. This exposes and corrects the earlier oracle error. Source 4 native scope is dispatch, and source code confirms process -> dispatch -> commit. Apply the published dispatch erratum instead of the erroneous frozen prohibition.

## qwen3.5-4b-q4 / decomp.commit-checks.read-only/answer_first

Does a conflict-free read-only commit create a new version or a write-history entry?

Native checks: True.

- supported: No: publish records a read-only commit and returns without creating a version or history entry; validation has already run.

````json
{
  "answer": "A conflict-free read-only commit does not create a new version or a write-history entry. According to the `publish` method in `src/interpreter/stm/committer.rs`, if the `written_vars` list is empty (indicating a read-only changeset), the method immediately returns after recording a `read_only_commit` statistic, skipping all logic that creates a new version or inserts into the write history.",
  "status": "answered",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/stm/committer.rs:611-709

Correct empty-write early return: no new version, snapshot application or write-history insertion, after the read-only statistic. Correct source 5.

## qwen3.5-4b-q4 / decomp.commit-checks.effects/answer_first

When does the attempt runner deliver physical effects, and what does it do after a rejected commit?

Native checks: True.

- supported: Effects are delivered only after an Ok commit result.
- supported: A rejected commit records the conflict and retries a fresh attempt with admission/backoff subject to the evaluation deadline.

````json
{
  "answer": "The attempt runner delivers physical effects immediately upon a successful commit, within the `run` loop. Specifically, after `body.commit_phase` returns `Ok(commit)`, the code checks if `commit` is `Ok(())`. If so, it calls `body.deliver(effects)` to flush the effects to the global state before breaking the loop. If the commit is rejected (returns `Err(conflict)`), the runner does not deliver effects. Instead, it increments the conflict counter, potentially sets a `retry_cell` for read-invalidated conflicts, and then waits for a backoff delay before retrying the attempt.",
  "status": "answered",
  "citations": [
    1,
    2
  ]
}
````

- [1] src/interpreter/stm/retry.rs:277-416
- [2] src/interpreter/task/mod.rs:1054-1083

Correct accepted-commit delivery, no effects on rejected attempts, conflict tracking and retry. Accept ordinary retry/backoff summary without requiring every admission/deadline spelling; it does not claim the loop breaks on rejection. Task and runner references 1/2 support the requested lifecycle.

## qwen3.5-4b-q4 / decomp.commit-checks.base-pin/answer_first

When is the base-version pin released during commit processing, and when may old history be evicted?

Native checks: True.

- supported: When releases_base is true, the base pin is released after commit validation, including rejection.
- supported: Old history is evicted only when live_versions is empty.

````json
{
  "answer": "The base-version pin is released immediately after the `commit` method returns successfully, provided the `releases_base` flag is true. This occurs within the `Committer::dispatch` handler for `CommitProtocol::Commit`, which calls `self.release(base)` unconditionally if `releases_base` is set. Old history may be evicted only if the committer's `live_versions` list becomes empty after processing the commit, triggering a call to `self.evict_old_versions()`. This eviction logic is gated by the state of `live_versions` and occurs post-commit validation.",
  "status": "answered",
  "citations": [
    4
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321

Correct dispatch handler, releases_base condition, post-validation release and live_versions-empty eviction, all in source 4. Read returns successfully as normal completion of the validation call, not an only-Ok restriction, because the next sentence explicitly says release is unconditional when flagged. Source also releases after Err. Apply the dispatch oracle erratum; naming the actual handler is correct.
