# Inspect the source-ranking trial

For the maintainer comparing search orders. These are exact source cards from the pinned
lpc-rs revision `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. All cards remain available in every order.
The model supplies labels only; it supplies no answer or explanation. Grades in the separate
audit are preliminary source-aware judgments, not a model confidence score.

## where are new transactions created?

| Order | First three source labels |
| --- | --- |
| Baseline | [1](#transaction-creation-1), [2](#transaction-creation-2), [3](#transaction-creation-3) |
| Model, forward presentation | [3](#transaction-creation-3), [1](#transaction-creation-1), [5](#transaction-creation-5) |
| Model, reversed presentation | [7](#transaction-creation-7), [6](#transaction-creation-6), [1](#transaction-creation-1) |

Missing from this pool: Task::open_attempt, the normal owning-task transaction creation caller, is absent.

<a id="transaction-creation-1"></a>

### 1. src/command/presence/transaction_tests.rs:20–25

Scope: `Pending::new`. Role hint: `code`.

```
async fn new(vm: &Vm) -> Self {
        let live = start_txn(&vm.global_state.committer_tx).await.unwrap();
        let mut template = TaskTemplate::from(vm.global_state.clone());
        template.txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        Self { live, template }
    }
```

<a id="transaction-creation-2"></a>

### 2. src/interpreter/stm/mod.rs:718–721

Scope: `TxnHandle::new`. Role hint: `code`.

```
pub(crate) fn new(txn: Transaction) -> Self {
        let mode = txn.snapshot.null_mode;
        Self(Arc::new(spin::Mutex::new(txn)), None, mode)
    }
```

<a id="transaction-creation-3"></a>

### 3. src/interpreter/stm/mod.rs:127–139

Scope: `Transaction`. Role hint: `code`.

```
impl Transaction {
    pub(crate) fn new(snapshot: Snapshot) -> Self {
        let version = snapshot.version();
        Self {
            snapshot,
            changeset: Changeset::new(version),
            effects: Vec::new(),
            pending_call_outs: Vec::new(),
            cancelled_call_outs: HashSet::new(),
            joinable: true,
            interrupted: None,
            compilation: diagnostics::CompilationTiming::default(),
            presence_revision: 0,

```

<a id="transaction-creation-4"></a>

### 4. doc/efun/transaction_stats.md:28–31

Role hint: `guide`.

```
| `backoff_cap_expiries` | Sleep waits that ran their full cap |
| `commits` | Successful commits that created a new world version |
| `read_only_commits` | Successful commits with no writes and no new version |
| `commit_conflicts` | Commit requests rejected due to 
```

<a id="transaction-creation-5"></a>

### 5. src/interpreter/stm/mod.rs:385–395

Scope: `Transaction::with_class_cow`. Role hint: `code`.

```
            return Ok(f(Arc::make_mut(own)));
        }
        let current = self
            .read_class(var_id)
            .ok_or_else(|| LpcError::runtime("class is not in this transaction's world"))?;
        let mut clone = (*current).clone();
        let result = f(&mut clone);
        self.changeset
            .write(var_id, WorldValue::Class(Arc::new(clone)));
        Ok(result)
    }
```

<a id="transaction-creation-6"></a>

### 6. AGENTS.md:131–139

Role hint: `policy`.

```
under `local/`; the archived Lima port history preserves existing records rather
than serving as a log for new work.

## Architecture facts that constrain changes

- **The STM committer is the sole serialization control.** There is no GIL, no
  write-write adjudication, and no global lock. Transactions (attempts) re-run on
  conflict via the attempt runner in `interpreter/stm/retry.rs`.
- **Object creation has two explicit placements.** Compile is one shared core

```

<a id="transaction-creation-7"></a>

### 7. src/interpreter/process/mod.rs:68–82

Scope: `Process`. Role hint: `code`.

```
pub struct Process {
    initial_image: Arc<ProgramImage>,

    /// The transaction-visible program and global-cell layout after an upgrade.
    pub(crate) image_cell: SVar<ProgramImage>,

    /// The second this process was constructed: what `object_time` answers.
    pub created: i64,

    /// Present only for nonresident programs defining the idle cleanup apply.
    pub(crate) cleanup: Option<Box<cleanup::Cleanup>>,

    /// How this process is named.
    name: ObjectName,


```

<a id="transaction-creation-8"></a>

### 8. doc/transaction-diagnostics.md:113–119

Role hint: `guide`.

```
rejected retry rejoins the tail of its newly reported cell's queue. Nested
applies share their owner's transaction and turn. Queues disappear when their
last participant leaves; no room configuration or mudlib hint is required.

Admission waiting uses the original execution allowance. Expiration while
queued records no new attempt or snapshot. Other conflict kinds retain backoff,
and the committer still validates every attempt. First attempts run immediately

```

## how do i add a new efun?

| Order | First three source labels |
| --- | --- |
| Baseline | [1](#add-efun-1), [2](#add-efun-2), [3](#add-efun-3) |
| Model, forward presentation | [1](#add-efun-1), [2](#add-efun-2), [3](#add-efun-3) |
| Model, reversed presentation | [7](#add-efun-7), [6](#add-efun-6), [1](#add-efun-1) |

Missing from this pool: The efuns! registration table/macro and module declaration are absent. Test and documentation extension locations are absent.

<a id="add-efun-1"></a>

### 1. src/interpreter/efun/parse_add_synonym.rs:14–24

Scope: `parse_add_synonym`. Role hint: `code`.

```
pub fn parse_add_synonym<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {
    let (LpcRef::String(new_verb), LpcRef::String(old_verb)) =
        (context.arg(0).clone(), context.arg(1).clone())
    else {
        return Err(context.runtime_error("parse_add_synonym: the verbs must be strings"));
    };
    let rule_filter = context
        .try_arg(2)
        .filter(|r| !r.is_null_in(context.txn()))
        .map(|r| match r {
            LpcRef::String(s) => Ok(s.to_str()),

```

<a id="add-efun-2"></a>

### 2. src/interpreter/efun/parse_add_synonym.rs:7–14

Role hint: `code`.

```
    interpreter::{efun::efun_context::EfunContext, lpc_ref::LpcRef},
};

/// `parse_add_synonym(new_verb, old_verb, rule)`: copies every rule
/// `this_object()` registered under `old_verb` (or only the one whose text
/// is `rule`) as a fresh rule for `new_verb` sharing its handlers; `old_verb`
/// is the typed verb, so a synonym of a synonym works.
pub fn parse_add_synonym<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {

```

<a id="add-efun-3"></a>

### 3. src/interpreter/efun/efun_context.rs:80–86

Scope: `EfunContext<'task, N>::new`. Role hint: `code`.

```
pub fn new(
        stack: &'task mut CallStack<N>,
        task_context: &'task TaskContext,
        efun: Efun,
    ) -> Self {
        Self::build(stack, task_context, efun, SmallVec::new(), None, None)
    }
```

<a id="add-efun-4"></a>

### 4. src/interpreter/efun/parse_add_rule.rs:14–23

Scope: `parse_add_rule`. Role hint: `code`.

```
pub fn parse_add_rule<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {
    let this = context.process().clone();
    if this.parser_ready.get().is_none() {
        return Err(context.runtime_error("parse_add_rule: parse_init() has not been called"));
    }
    let (LpcRef::String(verb), LpcRef::String(rule)) =
        (context.arg(0).clone(), context.arg(1).clone())
    else {
        return Err(context.runtime_error("parse_add_rule: the verb and rule must be strings"));
    };

```

<a id="add-efun-5"></a>

### 5. src/interpreter/efun/add_rule.rs:13–21

Scope: `add_rule`. Role hint: `code`.

```
pub fn add_rule<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {
    let Some(player) = context.this_player().load_full() else {
        return Err(context.runtime_error("add_rule: no this_player() to attach to"));
    };
    if !player.commands_enabled(context.txn()) {
        return Err(context.runtime_error("add_rule: this_player() is not a living object"));
    }

    let LpcRef::String(pattern) = context.arg(0).clone() else {

```

<a id="add-efun-6"></a>

### 6. src/interpreter/efun/add_action.rs:20–29

Scope: `add_action`. Role hint: `code`.

```
pub fn add_action<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {
    let Some(player) = context.this_player().load_full() else {
        return Err(context.runtime_error("add_action: no this_player() to attach to"));
    };
    if !player.commands_enabled(context.txn()) {
        return Err(context.runtime_error("add_action: this_player() is not a living object"));
    }

    let handler = handler_from(context, context.arg(0).clone(), "add_action")?;


```

<a id="add-efun-7"></a>

### 7. specs/terminal-colour/TECH.md:7–15

Role hint: `policy`.

```
input events are dispatched. query_connection constructs a fresh STM mapping.
Efuns are appended to the registry to preserve CallEfun indices. LPC callbacks
use Continuation rather than nesting host-language execution.

## Changes

- Add terminal capability state and ColourDepth to lpc-rs-telnet. A bounded
  TTYPE exchange reuses the RFC 1143 table and wire framing. No new socket path
  or LPC callback is needed for negotiation.

```

<a id="add-efun-8"></a>

### 8. doc/driver-comparison.md:698–698

Role hint: `guide`.

```
| Synonyms | The implementation accepts `parse_add_synonym(new, old, rule)` despite its reference showing two arguments. | Also supports the optional rule filter. Native synonyms copy current owner registrations, normalize verb-key whitespace, and keep exact rule-text filters. Review chaining and removal semantics in [`parse_add_synonym`](efun/parse_add_synonym.md) and [`parse_remove`](efun/parse_remove.md). |

```

## show me the codepath that transaction commits take, with all the checks they go through

| Order | First three source labels |
| --- | --- |
| Baseline | [1](#commit-checks-1), [2](#commit-checks-2), [3](#commit-checks-3) |
| Model, forward presentation | [3](#commit-checks-3), [4](#commit-checks-4), [5](#commit-checks-5) |
| Model, reversed presentation | [1](#commit-checks-1), [2](#commit-checks-2), [3](#commit-checks-3) |

Missing from this pool: The normal Task submission/delivery path is absent. Committer::dispatch and the validator with the requested rejection checks are absent.

<a id="commit-checks-1"></a>

### 1. doc/transaction-town/README.md:1–4

Role hint: `guide`.

```
# Transaction Town

An offline, interactive explanation of transactions in the LPC driver. Owning task A takes a snapshot, reads cells, stages private changes and output, then submits to the single committer. Another task B can commit in the meantime. A rejected attempt takes the return road through retry admission and starts again; an accepted attempt delivers its receipt.


```

<a id="commit-checks-2"></a>

### 2. doc/efun/get_char.md:27–32

Role hint: `guide`.

```

Registrations replace any pending `get_char` or `input_to` handler; the last
registration wins. They take effect in order with other socket operations
after the transaction commits. An aborted attempt registers nothing; conflict
retries publish only the successful attempt. A caught error preserves a
registration when its 
```

<a id="commit-checks-3"></a>

### 3. doc/collector-town/js/world.js:27–28

Role hint: `code`.

```
      tag: 'Check snapshot pins', short: 'Even one held snapshot sends collection down the refusal road.',
      body: 'The committer sums live snapshot pins across all versions. With 0 pins it can trace and sweep safely. With even 1 pin it replies GcRefused immediately, before changing the world. It does not wait for that transaction or take a global lock. Try raising “Snapshot pins” and watch the cart turn left. The accepted road that follows expands one atomic committer operation for teac
```

<a id="commit-checks-4"></a>

### 4. doc/command-rules.md:187–194

Role hint: `guide`.

```
or run timing alongside a build.

Validation: `cargo build --workspace`, `cargo test --workspace` (3,501 passed,
2 ignored), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `RUSTDOCFLAGS="-D warnings" cargo doc --workspace` passed.
Coverage includes concurrent blind registrations, ID/commit ordering,
same-transaction reads and edits, owner-authorized removal, synonyms, rollback,
parser edits from declining actor handlers, successful actor commands avoiding

```

<a id="commit-checks-5"></a>

### 5. doc/transaction-town/js/ui.js:352–365

Role hint: `code`.

```
n,
    resetAll: resetAll,
    paint: paint,
    showDistrict: showDistrict,
    unpin: unpin,
    activeDistrict: activeDistrict,
    takeFlyTo: function () {
      var p = fly;
      fly = null;
      return p;
    },
    closeAbout: closeAbout
  };
})(window);

```

<a id="commit-checks-6"></a>

### 6. src/interpreter/process/util.rs:9–19

Scope: `AllEnvironment`. Role hint: `code`.

```
pub struct AllEnvironment {
    /// The current environment. Calling `next` will return the environment of this `Process`.
    current: Option<Arc<Process>>,

    /// Already returned, by filename; the second line behind `move_to`'s
    /// refusal.
    seen: HashSet<Arc<Process>>,

    /// The transaction whose view this chain is followed through.
    txn: TxnHandle,
}
```

<a id="commit-checks-7"></a>

### 7. src/interpreter/sockets/transport.rs:1007–1020

Scope: `AttemptBody for Publication::commit_phase`. Role hint: `code`.

```
async fn commit_phase(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
        _live: LiveSnapshot,
    ) -> Result<(std::result::Result<(), Conflict>, Vec<Effect>)> {
        let txn = self
            .txn
            .take()
            .ok_or_else(|| error("missing native socket attempt"))?;
        Ok((
            commit_changeset(tx, txn.with(Transaction::take_changeset)).await?,
            Vec::new(),
        ))
    }
```

<a id="commit-checks-8"></a>

### 8. src/interpreter/sockets/transport/tests.rs:49–55

Scope: `Fixture::commit`. Role hint: `code`.

```
async fn commit(&self, txn: &TxnHandle, mut live: LiveSnapshot) {
        live.disarm();
        commit_changeset(&self.gs.committer_tx, txn.with(Transaction::take_changeset))
            .await
            .unwrap()
            .unwrap();
    }
```
