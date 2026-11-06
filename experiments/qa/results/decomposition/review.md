# Focused-question/output-order review

For evaluators. Contains gold facts; never index this artifact or send it to candidates. Agent-authored focused questions and source-aware grading are not independent acceptance.

## decomp.add-efun.full/status_first

how do i add a new efun?

broad; native checks: True; actual order: status, answer, citations.

- missing: Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- missing: Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- missing: Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the `abs` efun implementation and its usage in the `lpc-rs` interpreter, but they do not contain instructions or examples for adding a new efun. The `efuns!` macro and documentation are present, but no guide on how to define a new efun's prototype, register it in `EFUN_PROTOTYPES`, or compile it is included in the text.",
  "citations": []
}
````


Repeats the prior false refusal on full relevant source; no requested extension procedure is provided.

Unsupported: Claims no prototype/registration examples or guidance are supplied despite the complete macro documentation and registration examples.

## decomp.add-efun.full/answer_first

how do i add a new efun?

broad; native checks: True; actual order: answer, status, citations.

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

## decomp.add-efun.handler/answer_first

What Rust context/interface does a native efun handler use to read an argument and return a result?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.add-efun.handler/status_first

What Rust context/interface does a native efun handler use to read an argument and return a result?

narrow; native checks: True; actual order: status, answer, citations.

- supported: An EfunContext supplies arg(index) and return_efun_result(value); the Rust handler returns Result<()>.

````json
{
  "status": "answered",
  "answer": "A native efun handler uses the `EfunContext` interface, which provides methods like `arg` to read arguments and `return_efun_result` to return results. The specific implementation for the `abs` function is found in `src/interpreter/efun/abs.rs`.",
  "citations": [
    1
  ]
}
````

- [1] src/interpreter/efun/abs.rs:1-69

Correct EfunContext argument/result methods and example handler location. The unasked Rust Result<()> spelling is not required.

## decomp.add-efun.registration/status_first

Where is a native efun signature registered, and does the same registration also generate its runtime dispatch?

narrow; native checks: True; actual order: status, answer, citations.

- missing: Edit an efuns! table row in src/interpreter/efun/mod.rs, not a separate hand-maintained prototype map.
- contradicted: The macro generates both EFUN_PROTOTYPES entries and runtime dispatch from those rows.

````json
{
  "status": "answered",
  "answer": "The native efun signature for `abs` is registered in `src/interpreter/efun/mod.rs` at line 759 under the `abs` entry in the `EFUN_PROTOTYPES` static map. The same registration generates its runtime dispatch via the `call_efun` function, which matches the efun name against the `EFUN_PROTOTYPES` map to invoke the corresponding `abs` function (e.g., `abs` or `abs_in_module`).",
  "citations": [
    5,
    6
  ]
}
````

- [5] src/interpreter/efun/mod.rs:759-763
- [6] doc/efun/abs.md:1-11

Locates the abs row/file but does not distinguish the efuns! source row from the generated prototype map. The claimed runtime dispatch mechanism is wrong.

Unsupported: Misdescribes call_efun as matching names against EFUN_PROTOTYPES rather than dispatching on generated Efun enum variants.; Invents an abs_in_module function name.

## decomp.add-efun.registration/answer_first

Where is a native efun signature registered, and does the same registration also generate its runtime dispatch?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.transaction-creation.full/answer_first

where are new transactions created?

broad; native checks: True; actual order: answer, status, citations.

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

## decomp.transaction-creation.full/status_first

where are new transactions created?

broad; native checks: True; actual order: status, answer, citations.

- supported: The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- contradicted: A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created in the `Transaction::new` method within the `src/interpreter/stm/mod.rs` file. This method is called from `start_txn` (line 377-442 in `src/interpreter/task/mod.rs`), which is invoked when a `CommitProtocol::Start` message is received from the committer. The `Transaction::new` function initializes the transaction with the current snapshot version, creates a new `Changeset`, and sets up the `effects` and `pending_call_outs` vectors. This process is triggered during the execution of `begin_attempt` in `src/command/command_task.rs` (line 84-108), where `start_txn` is awaited to obtain a `LiveSnapshot`, which is then wrapped into a `TxnHandle` for the command task.",
  "citations": [
    1,
    3,
    5
  ]
}
````

- [1] src/interpreter/stm/mod.rs:127-143
- [3] src/interpreter/stm/retry.rs:477-494
- [5] src/command/command_task.rs:84-108

Same correct constructor/init detail and wrong task/message linkage as the prior long status-first control.

Unsupported: Attributes start_txn to Task::open_attempt source lines and reverses the Start request direction.

## decomp.transaction-creation.task-entry/status_first

Which method on Task creates a fresh transaction for a non-joining attempt?

narrow; native checks: True; actual order: status, answer, citations.

- supported: Task::open_attempt obtains a LiveSnapshot and assigns TxnHandle::new(Transaction::new(...)) to its context.

````json
{
  "status": "answered",
  "answer": "The method `open_attempt` on `Task` creates a fresh transaction for a non-joining attempt. It calls `start_txn` to begin the transaction, which in turn invokes `request` to send a `CommitProtocol::Start` message to the committer. This process establishes a new `Transaction` instance within the `LiveSnapshot` returned to the caller.",
  "citations": [
    3,
    4,
    5
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-494
- [4] src/interpreter/stm/committer.rs:289-301
- [5] src/command/command_task.rs:84-108

Correct requested method name. The added lifecycle explanation is false and the cited sources omit the main method block (source 2).

Unsupported: Says the returned LiveSnapshot contains a newly established Transaction instance; the caller constructs the Transaction from its snapshot.

## decomp.transaction-creation.task-entry/answer_first

Which method on Task creates a fresh transaction for a non-joining attempt?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.transaction-creation.start-message/answer_first

In start_txn, which side sends CommitProtocol::Start, and what comes back from the committer?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.transaction-creation.start-message/status_first

In start_txn, which side sends CommitProtocol::Start, and what comes back from the committer?

narrow; native checks: True; actual order: status, answer, citations.

- supported: The caller/client helper start_txn sends Start through the committer channel.
- supported: Committer::process handles Start and returns a LiveSnapshot of its current world.

````json
{
  "status": "answered",
  "answer": "In `start_txn`, the `request` function sends `CommitProtocol::Start` via a `flume::Sender<CommitProtocol>`. The committer responds by sending back a `LiveSnapshot` containing the new transaction state.",
  "citations": [
    3,
    4
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-494
- [4] src/interpreter/stm/committer.rs:289-301

Correct request sender/channel and LiveSnapshot reply, directly supported by sources 3/4. Interpret the loose new-transaction-state phrase as the world snapshot supplied to the attempt, not an explicit claim of a newly allocated Transaction object.

## decomp.commit-checks.full/status_first

show me the codepath that transaction commits take, with all the checks they go through

broad; native checks: True; actual order: status, answer, citations.

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
  "status": "answered",
  "answer": "The transaction commit codepath in `src/interpreter/stm/committer.rs` proceeds as follows: First, it validates the changeset against the current snapshot version, checking for future versions, history unavailability, and read invalidation conflicts. If the changeset folds successfully, it publishes the new version, updates the write history, and sends the commit result via the `Committer::dispatch` handler. The `commit` function specifically handles the `Commit` protocol message, which includes the changeset, a flag to release the base version, and a reply channel. Upon receiving the reply, it sends the result back to the client, optionally releasing the base version if `releases_base` is true, and updates internal statistics like `commit_service_ns` and `stats.commit`.",
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

Same partial gate summary and wrong caller/handler directions as the prior long status-first control. Omits merge/type rejection, read-only behavior, delivery/retry and pin-release/eviction ordering.

Unsupported: Invents Committer::dispatch and conflates process message handling with commit validation.; Describes the committer receiving a reply rather than a request.

## decomp.commit-checks.full/answer_first

show me the codepath that transaction commits take, with all the checks they go through

broad; native checks: True; actual order: answer, status, citations.

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

Similar three-gate partial summary to status-first. The submission chain and lifecycle branches remain incomplete; output ordering does not correct the broad trace.

Unsupported: Invents Committer::dispatch and attributes receive/release behavior to the wrong functions.

## decomp.commit-checks.future-base/answer_first

What happens if a changeset base version is newer than the committer current version?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.future-base/status_first

What happens if a changeset base version is newer than the committer current version?

narrow; native checks: False; actual order: status, answer, citations.

- supported: Committer::commit rejects it with Conflict::FutureVersion before publication.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain information about the behavior when a changeset's base version is newer than the committer's current version. While `src/interpreter/stm/committer.rs` (lines 611-613) explicitly handles the case where `current_version < changeset_version` by returning an error `Conflict::FutureVersion`, the specific consequences or alternative behaviors for the scenario where `changeset_version > current_version` are not described in the supplied text. The code only shows the error path for this condition, but not what happens if the condition is reversed.",
  "citations": [
    6
  ]
}
````

- [6] unavailable

Correct FutureVersion rejection appears in the narrative, followed by a contradictory refusal. It also cites unavailable label 6.

Unsupported: Treats current_version < changeset_version and changeset_version > current_version as different/reversed conditions, then claims the consequence is unknown.

Native rejection: Citation label is unavailable

## decomp.commit-checks.old-base/status_first

What happens if a changeset base version is older than the retained write history?

narrow; native checks: True; actual order: status, answer, citations.

- supported: It is rejected as Conflict::HistoryUnavailable because the required validation history is unavailable.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the logic for handling a conflict when a changeset's base version is older than the retained write history, but do not explicitly state the outcome. Specifically, the `commit` function in `src/interpreter/stm/committer.rs` (lines 611-613) returns an error `Conflict::HistoryUnavailable` if `changeset_version < self.oldest_retained`. However, the question asks what happens if the base version is *older* than the retained history, which implies `changeset_version < self.oldest_retained`. While the code returns an error in this specific case, the provided text does not explicitly define the *consequence* or the specific error type beyond the generic `Conflict::HistoryUnavailable` mentioned in the snippet. Without additional context on whether this error halts the transaction or if it is handled differently, the exact behavior cannot be fully determined from the supplied text alone.",
  "citations": [
    3
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523

Correct error content followed by a false refusal. Supplied code shows rejection and the runner outcome; cited helper 3 is not the validation block.

Unsupported: Claims the consequence/error outcome is not stated while explicitly describing return Err(Conflict::HistoryUnavailable).

## decomp.commit-checks.old-base/answer_first

What happens if a changeset base version is older than the retained write history?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.read-conflict/answer_first

Which committed writes are checked for invalidated reads, and what happens when one is found?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.read-conflict/status_first

Which committed writes are checked for invalidated reads, and what happens when one is found?

narrow; native checks: True; actual order: status, answer, citations.

- missing: Write-history versions strictly after the base through the current version are scanned; the base itself is skipped.
- supported: A conflicting tracked read rejects the changeset as ReadInvalidated before publication.

````json
{
  "status": "answered",
  "answer": "The code checks for invalidated reads by iterating through the `write_history` range from the changeset version to the current version. If a conflicting read is found, it returns an error with the conflict type `Conflict::ReadInvalidated`, which includes details about the conflicting read cell, the base version, the current version, and the writer information.",
  "citations": [
    4
  ]
}
````

- [4] src/interpreter/stm/committer.rs:302-321

Correct rejection/error fields, but does not establish that the base version itself is skipped; repeating the syntactic range is not the requested effective set of writes checked. It cites message handler 4 rather than validation block 5.

## decomp.commit-checks.merge-check/status_first

How are merge operations checked before publication, and what happens on a mismatch?

narrow; native checks: True; actual order: status, answer, citations.

- contradicted: fold_merges uses the currently committed values.
- supported: A merge/type mismatch rejects the entire changeset as MergeMismatch before any changes apply.

````json
{
  "status": "answered",
  "answer": "Merge operations are checked by iterating through the `write_history` range from the changeset version up to the current version. If a conflicting read is found, the transaction returns an error with `Conflict::ReadInvalidated`. If the merge fails (e.g., type mismatch), the transaction returns an error with `Conflict::MergeMismatch`. In both cases, the `stats.conflict()` counter is incremented, and the commit is rejected.",
  "citations": [
    3,
    4
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523
- [4] src/interpreter/stm/committer.rs:302-321

Correct MergeMismatch rejection, wrong explanation of the merge-check operation and references to helpers/handler rather than gate source 5.

Unsupported: Describes merge checking through the write-history/read-conflict scan rather than folding merges against committed values.

## decomp.commit-checks.merge-check/answer_first

How are merge operations checked before publication, and what happens on a mismatch?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.commit-receiver/answer_first

Which Committer method receives CommitProtocol::Commit, and which method does it call to validate the changeset?

narrow; native checks: True; actual order: answer, status, citations.

- contradicted: Committer::process handles the Commit message and calls self.commit(changeset), the Committer::commit validation method.

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

Correct downstream commit validator name does not fix the explicitly requested receive-method ownership error.

Unsupported: Invents Committer::dispatch; the supplied receiver method is Committer::process.

## decomp.commit-checks.commit-receiver/status_first

Which Committer method receives CommitProtocol::Commit, and which method does it call to validate the changeset?

narrow; native checks: True; actual order: status, answer, citations.

- contradicted: Committer::process handles the Commit message and calls self.commit(changeset), the Committer::commit validation method.

````json
{
  "status": "answered",
  "answer": "The `Committer::dispatch` method receives `CommitProtocol::Commit`. It calls the `commit` method to validate the changeset, which checks version history and applies conflict rules before publishing the new snapshot.",
  "citations": [
    3,
    4
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523
- [4] src/interpreter/stm/committer.rs:302-321

Same receiver-name error as answer-first despite the direct method-ownership question and native scope metadata.

Unsupported: Invents Committer::dispatch instead of the supplied Committer::process receiver.

## decomp.commit-checks.read-only/status_first

Does a conflict-free read-only commit create a new version or a write-history entry?

narrow; native checks: True; actual order: status, answer, citations.

- contradicted: No: publish records a read-only commit and returns without creating a version or history entry; validation has already run.

````json
{
  "status": "answered",
  "answer": "A conflict-free read-only commit creates a new version but does not create a write-history entry. This is because the `publish` method, which handles the commit logic, explicitly checks if `written_vars` is empty. If it is, it logs a `read_only_commit` and returns early without inserting anything into `self.write_history`. The version is still created via `new_version = Version::new()` and applied to the snapshot, but the history insertion is conditional on the presence of touched variables.",
  "citations": [
    3
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523

Correct absence of history insertion does not fix the incorrect creation of a version. Cites helper 3 rather than publish source 5.

Unsupported: Claims Version::new and snapshot.apply still execute for a read-only changeset, despite the earlier empty-write return.

## decomp.commit-checks.read-only/answer_first

Does a conflict-free read-only commit create a new version or a write-history entry?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.effects/answer_first

When does the attempt runner deliver physical effects, and what does it do after a rejected commit?

narrow; native checks: True; actual order: answer, status, citations.

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

## decomp.commit-checks.effects/status_first

When does the attempt runner deliver physical effects, and what does it do after a rejected commit?

narrow; native checks: True; actual order: status, answer, citations.

- supported: Effects are delivered only after an Ok commit result.
- contradicted: A rejected commit records the conflict and retries a fresh attempt with admission/backoff subject to the evaluation deadline.

````json
{
  "status": "answered",
  "answer": "The attempt runner delivers physical effects via the `deliver` method, which is called after a successful commit phase. After a rejected commit, the runner breaks the loop, records the conflict count, and triggers a backoff mechanism (either `watching` or `new`) to retry the transaction before attempting to deliver the effects again.",
  "citations": [
    1,
    5
  ]
}
````

- [1] src/interpreter/stm/retry.rs:277-416
- [5] src/interpreter/stm/committer.rs:611-709

Accept successful commit phase as ordinary shorthand for accepted commit for the first requested fact. The rejected-commit loop behavior is wrong.

Unsupported: Says the runner breaks the loop after a rejected commit; conflicts instead continue to a new attempt via admission/backoff.

## decomp.commit-checks.base-pin/status_first

When is the base-version pin released during commit processing, and when may old history be evicted?

narrow; native checks: True; actual order: status, answer, citations.

- supported: When releases_base is true, the base pin is released after commit validation, including rejection.
- contradicted: Old history is evicted only when live_versions is empty.

````json
{
  "status": "answered",
  "answer": "The base-version pin is released during the `commit` phase of the commit protocol, specifically when the `releases_base` flag is set to `true` in the `CommitProtocol::Commit` message. This occurs after the commit logic validates the changeset (checking version history and conflict rules) and folds merges. Old history is evicted when the `oldest_retained` version is reached, which happens if the changeset's base version is older than the oldest retained version. This eviction check occurs in the `commit` method of the committer, specifically in the condition `if changeset_version < self.oldest_retained`.",
  "citations": [
    3,
    5
  ]
}
````

- [3] src/interpreter/stm/retry.rs:477-523
- [5] src/interpreter/stm/committer.rs:611-709

Correct release flag and after-validation phase at the requested level. Wrong eviction condition; it also fails to cite process block 4 where actual release/eviction happen.

Unsupported: Treats changeset_version < oldest_retained, a HistoryUnavailable rejection check, as the history-eviction trigger.

## decomp.commit-checks.base-pin/answer_first

When is the base-version pin released during commit processing, and when may old history be evicted?

narrow; native checks: True; actual order: answer, status, citations.

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

Correct release flag/phase and live_versions-empty eviction condition. The source also applies release/eviction after rejection; the success phrasing is incomplete but not read as an explicit only-success assertion. The invented handler and citations omitting actual process block 4 prevent task success.

Unsupported: Invents Committer::dispatch as the receiving handler.
