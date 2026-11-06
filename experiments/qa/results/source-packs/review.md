# Source-complete diagnostic review

For evaluators. Contains gold rubrics; never include this artifact in inference. Manual grading is by the source-aware pack/rubric author, not independent human acceptance.

## maintainer.lpc-rs.add-efun/retrieved_ids

how do i add a new efun?

Native binding: True; 2324 source bytes; 100 words / 384 output tokens allowed.

- missing: Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- missing: Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- missing: Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "answered",
  "answer": "To add a new efun, use `add_rule` with the pattern as the first argument and the handler function as the second. This compiles the pattern and registers the handler with the actor.",
  "citations": [
    3
  ]
}
````

- [3] src/interpreter/efun/add_rule.rs:13-28 (code)

Control repeats the target-language command-registration procedure; no native module/handler, efuns! table row, tests or documentation extension path is supplied.

Unsupported: Presents registering a command pattern/handler through add_rule as adding a native efun.

## maintainer.lpc-rs.add-efun/curated_ids

how do i add a new efun?

Native binding: True; 9026 source bytes; 100 words / 384 output tokens allowed.

- missing: Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- supported: Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- missing: Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "answered",
  "answer": "To add a new efun, define it using the `efuns!` macro in `src/interpreter/efun/mod.rs`. The macro requires a name, return type, arity (number of arguments), and argument types. For example, to add `abs`, you would define it with `returns: LpcType::Mixed`, `arity: 1`, and `args: [LpcType::Int(false) | LpcType::Float(false)]` as shown in the existing `abs` entry.",
  "citations": [
    5
  ]
}
````

- [5] src/interpreter/efun/mod.rs:759-763 (code)

Correctly finds the native efuns! registry and gives the existing abs signature example. It omits implementing/declaring the Rust handler and adding tests/documentation, so the full extension procedure is incomplete. Treat LpcType::Mixed as shorthand for the supplied variant rather than a complete compiling code example.

Unsupported: Says arity and argument types are required by the macro, although its documented syntax permits omitting them.

## maintainer.lpc-rs.add-efun/curated_long

how do i add a new efun?

Native binding: True; 9026 source bytes; 300 words / 1024 output tokens allowed.

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


Refuses with complete relevant source available. It generates only 106 tokens, below even the short output cap; no context overflow or truncation explains this failure.

Unsupported: Claims no prototype/registration examples or guidance are supplied despite the macro syntax documentation, generated prototype/dispatch definitions and registration rows.

## maintainer.lpc-rs.transaction-creation/curated_ids

where are new transactions created?

Native binding: True; 5024 source bytes; 100 words / 384 output tokens allowed.

- supported: The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- missing: A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created in the `Transaction::new` method within `src/interpreter/stm/mod.rs` at line 127, which takes a `Snapshot` and initializes a `Changeset` and `effects` vector.",
  "citations": [
    1
  ]
}
````

- [1] src/interpreter/stm/mod.rs:127-143 (code)

Correctly locates Transaction::new and its snapshot/Changeset initialization. Does not identify Task::open_attempt as the normal owning-task creation path, although that full method is supplied. Useful constructor lookup, incomplete under the frozen two-part rubric.

## maintainer.lpc-rs.transaction-creation/curated_long

where are new transactions created?

Native binding: True; 5024 source bytes; 300 words / 1024 output tokens allowed.

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

- [1] src/interpreter/stm/mod.rs:127-143 (code)
- [3] src/interpreter/stm/retry.rs:477-494 (code)
- [5] src/command/command_task.rs:84-108 (code)

Correct constructor, Changeset initialization and command begin_attempt example. The normal Task::open_attempt caller chain is misnamed/misdirected despite all relevant definitions being supplied. 201 tokens, below the original token cap.

Unsupported: Attributes a start_txn definition to Task::open_attempt source lines and reverses the Start message direction: the client sends Start to the committer, which returns a LiveSnapshot.

## maintainer.lpc-rs.transaction-creation/retrieved_ids

where are new transactions created?

Native binding: True; 1914 source bytes; 100 words / 384 output tokens allowed.

- missing: The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- contradicted: A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created in `src/command/presence/transaction_tests.rs` within the `new` function, which starts a transaction via `start_txn(&vm.global_state.committer_tx).await.unwrap()` and wraps it in a `TxnHandle`.",
  "citations": [
    2
  ]
}
````

- [2] doc/efun/load_object.md:8-24 (guide)

Control continues to select an unrecognised test-module filename and omits both Transaction::new location and the normal owning Task entry point. The available ID does not establish the production-role claim.

Unsupported: Presents the cfg(test)-gated Pending::new fixture in presence/transaction_tests.rs as the production transaction creation entry point.

## maintainer.lpc-rs.commit-checks/curated_long

show me the codepath that transaction commits take, with all the checks they go through

Native binding: True; 12993 source bytes; 300 words / 1024 output tokens allowed.

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

- [3] src/interpreter/stm/retry.rs:477-523 (code)
- [4] src/interpreter/stm/committer.rs:302-321 (code)
- [5] src/interpreter/stm/committer.rs:611-709 (code)

Names future-version, unavailable-history and read-invalidation gates. Acknowledges folding but omits merge/type rejection, the read-only publication branch, physical-effect/retry outcome and pin-release/eviction ordering. Generic write publication is not counted as the complete frozen publication fact. Only 200 tokens / 780 answer characters were used.

Unsupported: Invents Committer::dispatch and conflates Committer::process message handling with Committer::commit.; Describes the committer receiving a reply rather than processing a request and sending its result.

## maintainer.lpc-rs.commit-checks/retrieved_ids

show me the codepath that transaction commits take, with all the checks they go through

Native binding: True; 2419 source bytes; 100 words / 384 output tokens allowed.

- missing: The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- missing: Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- missing: A base version older than retained history is rejected as HistoryUnavailable.
- missing: Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- missing: Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- missing: Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- missing: Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- missing: The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the transaction commit process and checks (e.g., transactional cells, shared-array behavior, and retry mechanisms) but do not contain the specific codepath or implementation details of the commit logic itself. The text explains the *rules* and *consequences* of commits rather than the actual code execution flow.",
  "citations": []
}
````


Faithfully refuses on class/illustration excerpts that do not show the commit implementation. The repository task remains unanswered.

## maintainer.lpc-rs.commit-checks/curated_ids

show me the codepath that transaction commits take, with all the checks they go through

Native binding: True; 12993 source bytes; 100 words / 384 output tokens allowed.

- contradicted: The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- supported: Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- supported: A base version older than retained history is rejected as HistoryUnavailable.
- supported: Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- supported: Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- missing: Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- missing: Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- missing: The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "status": "answered",
  "answer": "The transaction commit codepath in `src/interpreter/stm/committer.rs` proceeds as follows: First, it validates the changeset version against the current snapshot version and the oldest retained version, returning a `Conflict` error if the version is too old or if the base version is in the future. Next, it iterates through the write history to check for conflicting reads, incrementing the conflict counter if found. It then folds merges to ensure type compatibility; if this fails, it returns a `Conflict` error. Finally, it publishes the changeset to the snapshot, updates the write history, and sends the commit result via the `Committer::dispatch` handler.",
  "citations": [
    1,
    2,
    3,
    4,
    5
  ]
}
````

- [1] src/interpreter/stm/retry.rs:277-416 (code)
- [2] src/interpreter/task/mod.rs:1054-1083 (code)
- [3] src/interpreter/stm/retry.rs:477-523 (code)
- [4] src/interpreter/stm/committer.rs:302-321 (code)
- [5] src/interpreter/stm/committer.rs:611-709 (code)

Accurately names future-base, unavailable-history, read-invalidation and merge/type rejection before publication. It omits the actual submission chain, read-only no-version branch, effect-delivery/retry and pin-release/eviction ordering. Normal write publication alone does not satisfy the full frozen publication fact.

Unsupported: Invents a Committer::dispatch handler; the supplied receiver is Committer::process, which invokes Committer::commit.
