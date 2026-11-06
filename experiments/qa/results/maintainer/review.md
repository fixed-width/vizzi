# Source-aware response review

For evaluators. Contains gold facts; never index this artifact or send it to candidates. Grading is source-aware agent work, not independent human acceptance.

## maintainer.lpc-rs.add-efun/legacy_quotes

how do i add a new efun?

Expected: answered. Native source checks: False.

- Fact 1 (missing): Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- Fact 2 (missing): Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- Fact 3 (missing): Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "answered",
  "answer": "To add a new efun, you must first register a parser rule using `parse_add_rule` with the efun name and a compiled rule string, ensuring `parse_init()` has been called on the object. Then, register the compiled action using `add_action` with the efun name and the compiled handler string.",
  "citations": [
    {
      "label": 2,
      "quote": "parse_add_rule(verb, rule): appends a verb-attached rule owned by this_object(), which must have called parse_init()."
    },
    {
      "label": 3,
      "quote": "add_rule, an efun that registers a pattern on this_player(): one rule per leading verb, all under the returned id."
    }
  ]
}
````

- [2] src/interpreter/efun/parse_add_rule.rs:1-28 (code)
- [3] src/interpreter/efun/add_rule.rs:1-52 (mixed)

No native handler/module, efuns! signature/dispatch registration, or tests/documentation extension path is supplied. Selected excerpts concern using existing efuns, not extending the native set.

Unsupported: Claims registering parser/command rules with parse_add_rule/add_rule/add_action adds a native efun.

Native rejection: Citation quote does not match supplied source

## maintainer.lpc-rs.add-efun/compact_quotes

how do i add a new efun?

Expected: answered. Native source checks: False.

- Fact 1 (missing): Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- Fact 2 (missing): Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- Fact 3 (missing): Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "answered",
  "answer": "To add a new efun, use `add_rule` with the pattern as the first argument and the handler function as the second. The handler must be registered via `ActorRules::new` within the transaction context.",
  "citations": [
    {
      "label": 3,
      "quote": "let handler = handler_from(context, context.arg(1).clone(), \"add_rule\")?"
    },
    {
      "label": 1,
      "quote": "pub fn add_rule<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {"
    }
  ]
}
````

- [3] src/interpreter/efun/add_rule.rs:13-28 (code)
- [1] src/interpreter/efun/parse_add_synonym.rs:14-31 (code)

No native handler/module, efuns! signature/dispatch registration, or tests/documentation extension path is supplied. Selected excerpts concern using existing efuns, not extending the native set.

Unsupported: Claims registering parser/command rules with parse_add_rule/add_rule/add_action adds a native efun.

Native rejection: Citation quote does not match supplied source

## maintainer.lpc-rs.add-efun/compact_ids

how do i add a new efun?

Expected: answered. Native source checks: True.

- Fact 1 (missing): Implement the native handler under src/interpreter/efun using EfunContext, return_efun_result and Result, and declare a new handler module when needed.
- Fact 2 (missing): Add an efuns! table row in src/interpreter/efun/mod.rs with return/argument types and arity, choosing async/grouped options when appropriate. The macro generates dispatch and EFUN_PROTOTYPES; these are not separate hand-maintained registrations.
- Fact 3 (missing): Add tests that exercise calls and errors, and add doc/efun/<name>.md documentation; efun Markdown files are automatically bundled for the language server.

````json
{
  "status": "answered",
  "answer": "To add a new efun, use `add_rule` with the pattern as the first argument and the handler function as the second. The handler must be registered via `ActorRules::new` within the transaction context.",
  "citations": [
    1,
    2,
    3
  ]
}
````

- [1] src/interpreter/efun/parse_add_synonym.rs:14-31 (code)
- [2] src/interpreter/efun/parse_add_rule.rs:14-28 (code)
- [3] src/interpreter/efun/add_rule.rs:13-28 (code)

No native handler/module, efuns! signature/dispatch registration, or tests/documentation extension path is supplied. Selected excerpts concern using existing efuns, not extending the native set.

Unsupported: Claims registering parser/command rules with parse_add_rule/add_rule/add_action adds a native efun.

## maintainer.lpc-rs.transaction-creation/compact_quotes

where are new transactions created?

Expected: answered. Native source checks: False.

- Fact 1 (missing): The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- Fact 2 (contradicted): A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created by the `start_txn` function, which is called within the `new` method of `TaskTemplate` (line 20-25). This method initiates a transaction using the `live` inner transaction from the global state before wrapping it in a new `TxnHandle` for the `TaskTemplate`.",
  "citations": [
    {
      "label": 3,
      "quote": "async fn new(vm: &Vm) -> Self {\\n        let live = start_txn(&vm.global_state.committer_tx).await.unwrap();\\n        //"
    }
  ]
}
````

- [3] src/command/presence/transaction_tests.rs:20-25 (code)

The filename heuristic misses _tests.rs and does not propagate the parent module cfg(test), so the selected test helper is incorrectly labelled code. No normal owning-task path is supplied.

Unsupported: Treats Pending::new in transaction_tests.rs, an externally cfg(test)-gated module, as the production transaction entry point.; Attributes Pending::new to TaskTemplate::new.

Native rejection: Citation quote does not match supplied source

## maintainer.lpc-rs.transaction-creation/compact_ids

where are new transactions created?

Expected: answered. Native source checks: True.

- Fact 1 (missing): The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- Fact 2 (contradicted): A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created by the `start_txn` function, which is called within the `new` function of `src/command/presence/transaction_tests.rs`. This function acquires a live transaction from the global state's committer, wraps it in a `TaskTemplate`, and assigns it to the `txn` field of the `TxnHandle`.",
  "citations": [
    2
  ]
}
````

- [2] doc/efun/load_object.md:8-24 (guide)

The filename heuristic misses _tests.rs and does not propagate the parent module cfg(test), so the selected test helper is incorrectly labelled code. No normal owning-task path is supplied.

Unsupported: Treats Pending::new in transaction_tests.rs, an externally cfg(test)-gated module, as the production transaction entry point.; Describes a txn field on TxnHandle rather than the TaskTemplate field, and cites unrelated load_object documentation at label 2.

## maintainer.lpc-rs.transaction-creation/legacy_quotes

where are new transactions created?

Expected: answered. Native source checks: False.

- Fact 1 (missing): The transaction constructor is Transaction::new(snapshot) in src/interpreter/stm/mod.rs; it starts a Changeset at that snapshot version.
- Fact 2 (missing): A normal owning Task creates a fresh transaction in Task::open_attempt in src/interpreter/task/mod.rs: start_txn obtains the current LiveSnapshot and the context receives TxnHandle::new(Transaction::new(live.inner.clone())).

````json
{
  "status": "answered",
  "answer": "New transactions are created when a player command is executed via `attach`, `exec`, or `detach` transactions, which write the connection cell in their own transaction to establish bindings for the program and global-cell layout.",
  "citations": [
    {
      "label": 1,
      "quote": "The cell is written by the attach/exec/destruct transactions; the connection's back-reference only by the binding module"
    }
  ]
}
````

- [1] src/interpreter/process/mod.rs:57-100 (code)

Does not locate Transaction::new or normal Task::open_attempt creation; the quote is also not literal.

Unsupported: Treats connection-cell writes in attach/exec/destruct paths as the transaction allocation entry point.

Native rejection: Citation quote does not match supplied source

## maintainer.lpc-rs.commit-checks/compact_ids

show me the codepath that transaction commits take, with all the checks they go through

Expected: answered. Native source checks: True.

- Fact 1 (missing): The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- Fact 2 (missing): Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- Fact 3 (missing): A base version older than retained history is rejected as HistoryUnavailable.
- Fact 4 (missing): Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- Fact 5 (missing): Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- Fact 6 (missing): Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- Fact 7 (missing): Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- Fact 8 (missing): The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the transaction commit process and snapshot mechanics but do not contain the specific codepath or detailed checks for a transaction commit. They mention that 'fields with the same name and declared type reuse storage' and that 'Access through a different compiled definition acquires a field view', but they do not list the actual validation checks performed during the commit phase.",
  "citations": []
}
````


Faithful refusal to invent a complete commit codepath from insufficient context. No submission chain, four validation gates, publication, delivery/retry, or pin-release ordering is established by the answer. The source-specific task remains unanswered.

## maintainer.lpc-rs.commit-checks/legacy_quotes

show me the codepath that transaction commits take, with all the checks they go through

Expected: answered. Native source checks: True.

- Fact 1 (missing): The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- Fact 2 (missing): Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- Fact 3 (missing): A base version older than retained history is rejected as HistoryUnavailable.
- Fact 4 (missing): Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- Fact 5 (missing): Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- Fact 6 (missing): Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- Fact 7 (missing): Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- Fact 8 (missing): The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts do not contain information about the codepath for transaction commits in the `lpc-rs` repository. While excerpts [1], [2], and [6] show JavaScript UI code and Lima port history, and excerpt [3] shows Rust test code, none of them describe the transaction commit logic or its associated checks.",
  "citations": []
}
````


Faithful refusal to invent a complete commit codepath from insufficient context. No submission chain, four validation gates, publication, delivery/retry, or pin-release ordering is established by the answer. The source-specific task remains unanswered.

## maintainer.lpc-rs.commit-checks/compact_quotes

show me the codepath that transaction commits take, with all the checks they go through

Expected: answered. Native source checks: True.

- Fact 1 (missing): The attempt runner calls the task/body commit_phase; it submits the changeset through retry::commit_changeset as CommitProtocol::Commit, which Committer::process sends to Committer::commit.
- Fact 2 (missing): Before publishing, a base version ahead of the current world is rejected as FutureVersion.
- Fact 3 (missing): A base version older than retained history is rejected as HistoryUnavailable.
- Fact 4 (missing): Validation scans writes after the base version and rejects invalidated reads as ReadInvalidated, skipping the base version itself.
- Fact 5 (missing): Merges are folded against committed values before any publication; a merge/type mismatch rejects the whole changeset as MergeMismatch.
- Fact 6 (missing): Only validated changes are published. Read-only changes create no new version; writes apply a new snapshot, record write history and notify commit watchers.
- Fact 7 (missing): Physical effects are delivered only after a successful commit; conflicts lead to a new attempt with admission/backoff and an evaluation deadline rather than publishing the rejected attempt.
- Fact 8 (missing): The commit processing path releases the disarmed base pin after validation, including rejection, and evicts history only when no live snapshots remain.

````json
{
  "status": "insufficient_evidence",
  "answer": "The provided excerpts describe the transaction flow and checks (e.g., role=guide, role=code, role=transaction) but do not contain the specific codepath for a successful commit or the detailed checks performed during that commit process.",
  "citations": []
}
````


Faithful refusal to invent a complete commit codepath from insufficient context. No submission chain, four validation gates, publication, delivery/retry, or pin-release ordering is established by the answer. The source-specific task remains unanswered.

Unsupported: Mentions a role=transaction annotation that is not one of the supplied role hints.
