# Inspect source coverage for the maintainer questions

For the maintainer comparing deterministic source search. All source cards below come
from the pinned lpc-rs revision `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. These are locations and excerpts,
not generated answers or a resolved call graph. Scope and literal matches are recorded
separately in the assessment. The compact control shares the new macro-aware index.

## where are new transactions created?

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| constructor | 3 | — | — |
| owning-entry | — | — | — |

### compact source cards

#### 1. src/command/presence/transaction_tests.rs:20–25

Scope: `Pending::new` (function). Role hint: `code`.

```
async fn new(vm: &Vm) -> Self {
        let live = start_txn(&vm.global_state.committer_tx).await.unwrap();
        let mut template = TaskTemplate::from(vm.global_state.clone());
        template.txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        Self { live, template }
    }
```

#### 2. src/interpreter/stm/mod.rs:718–721

Scope: `TxnHandle::new` (function). Role hint: `code`.

```
pub(crate) fn new(txn: Transaction) -> Self {
        let mode = txn.snapshot.null_mode;
        Self(Arc::new(spin::Mutex::new(txn)), None, mode)
    }
```

#### 3. src/interpreter/stm/mod.rs:127–139

Scope: `Transaction` (implementation). Role hint: `code`.

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

#### 4. doc/efun/transaction_stats.md:28–31

Role hint: `guide`.

```
| `backoff_cap_expiries` | Sleep waits that ran their full cap |
| `commits` | Successful commits that created a new world version |
| `read_only_commits` | Successful commits with no writes and no new version |
| `commit_conflicts` | Commit requests rejected due to 
```

#### 5. src/interpreter/stm/mod.rs:385–395

Scope: `Transaction::with_class_cow` (function). Role hint: `code`.

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

#### 6. AGENTS.md:131–139

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

#### 7. src/interpreter/process/mod.rs:68–82

Scope: `Process` (struct). Role hint: `code`.

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

#### 8. doc/transaction-diagnostics.md:113–119

Role hint: `guide`.

```
rejected retry rejoins the tail of its newly reported cell's queue. Nested
applies share their owner's transaction and turn. Queues disappear when their
last participant leaves; no room configuration or mudlib hint is required.

Admission waiting uses the original execution allowance. Expiration while
queued records no new attempt or snapshot. Other conflict kinds retain backoff,
and the committer still validates every attempt. First attempts run immediately

```

### search source cards

#### 1. src/interpreter/stm/mod.rs:718–721

Scope: `TxnHandle::new` (function). Role hint: `code`.

```
pub(crate) fn new(txn: Transaction) -> Self {
        let mode = txn.snapshot.null_mode;
        Self(Arc::new(spin::Mutex::new(txn)), None, mode)
    }
```

#### 2. src/interpreter/sockets/transport.rs:940–951

Scope: `AttemptBody for Publication::begin_attempt` (function). Role hint: `code`.

```
async fn begin_attempt(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
    ) -> Result<Option<LiveSnapshot>> {
        let live = start_txn(tx).await?;
        let txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        let access = SocketAccess {
            sockets: &self.gs.sockets,
            txn: txn.clone(),
        };
        self.outcome = match &self.update {
            Update::InputCapacity(handle) => Outcome::Capacity(access.input_capacity(*handle)?),

```

#### 3. src/interpreter/process/mod.rs:68–82

Scope: `Process` (struct). Role hint: `code`.

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

#### 4. src/command/command_task.rs:84–94

Scope: `AttemptBody for CommandTask::begin_attempt` (function). Role hint: `code`.

```
async fn begin_attempt(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
    ) -> Result<Option<LiveSnapshot>> {
        let live = start_txn(tx).await?;
        let mut template = self.template.clone();
        template.txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        template
            .txn
            .with(|txn| txn.set_origin(&self.actor, "command"));
        template.set_this_player(Some(self.actor.clone()));

```

#### 5. src/test_support/mod.rs:204–218

Scope: `task_at` (function). Role hint: `code`.

```
    let mut context =
        TaskTemplate::from(vm.global_state.clone()).into_task_context(process.clone());
    context.txn = TxnHandle::new(Transaction::new(live.inner.clone()));
    process.claim_init(&context.txn);
    let mut task = Task::new(context);
    let create = process
        .initial_program()
        .lookup_function("create")
        .unwrap()
        .clone();
    task.stack
        .push(CallFrame::new(
            process,
            create,
            0,

```

#### 6. src/interpreter/sockets/callbacks.rs:234–246

Scope: `AttemptBody for Invocation<'_>::begin_attempt` (function). Role hint: `code`.

```
async fn begin_attempt(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
    ) -> Result<Option<LiveSnapshot>> {
        self.lease = None;
        self.failure = None;
        self.task = None;
        self.txn = None;
        let live = start_txn(tx).await?;
        let txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        self.txn = Some(txn.clone());
        let Some(lease) = self.access().lease(self.event.handle)? else {
            return Ok(Some(live));

```

#### 7. src/interpreter/sockets/mod.rs:241–257

Scope: `Sockets::root` (function). Role hint: `code`.

```
        self.registry.get().copied()
    }

    fn registry(&self) -> VarId {
        *self.registry.get_or_init(VarId::new)
    }

    fn sender(&self) -> Result<&flume::Sender<SocketCommands>> {
        let sender = &self.queue().sender;
        if sender.is_disconnected() {
            Err(error("transport is unavailable"))
        } else {
            Ok(sender)
        }
    }

    fn cells(&self, txn: &mut Transaction, handle: SocketHandle) -> Option<Cells> {

```

#### 8. src/interpreter/sockets/callbacks.rs:354–365

Scope: `AttemptBody for Cleanup<'_>::begin_attempt` (function). Role hint: `code`.

```
async fn begin_attempt(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
    ) -> Result<Option<LiveSnapshot>> {
        let live = start_txn(tx).await?;
        let txn = TxnHandle::new(Transaction::new(live.inner.clone()));
        let access = SocketAccess {
            sockets: &self.gs.sockets,
            txn: txn.clone(),
        };
        if let Some(lease) = access.lease(self.event.handle)?
            && lease.event().id == self.event.id

```

## how do i add a new efun?

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| registration-definition | — | 1 | 1 |
| registration-table | — | — | — |

### compact source cards

#### 1. src/interpreter/efun/parse_add_synonym.rs:14–24

Scope: `parse_add_synonym` (function). Role hint: `code`.

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

#### 2. src/interpreter/efun/parse_add_synonym.rs:7–14

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

#### 3. src/interpreter/efun/efun_context.rs:80–86

Scope: `EfunContext<'task, N>::new` (function). Role hint: `code`.

```
pub fn new(
        stack: &'task mut CallStack<N>,
        task_context: &'task TaskContext,
        efun: Efun,
    ) -> Self {
        Self::build(stack, task_context, efun, SmallVec::new(), None, None)
    }
```

#### 4. src/interpreter/efun/parse_add_rule.rs:14–23

Scope: `parse_add_rule` (function). Role hint: `code`.

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

#### 5. src/interpreter/efun/add_rule.rs:13–21

Scope: `add_rule` (function). Role hint: `code`.

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

#### 6. src/interpreter/efun/add_action.rs:20–29

Scope: `add_action` (function). Role hint: `code`.

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

#### 7. specs/terminal-colour/TECH.md:7–15

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

#### 8. doc/driver-comparison.md:698–698

Role hint: `guide`.

```
| Synonyms | The implementation accepts `parse_add_synonym(new, old, rule)` despite its reference showing two arguments. | Also supports the optional rule filter. Native synonyms copy current owner registrations, normalize verb-key whitespace, and keep exact rule-text filters. Review chaining and removal semantics in [`parse_add_synonym`](efun/parse_add_synonym.md) and [`parse_remove`](efun/parse_remove.md). |

```

### search source cards

#### 1. src/interpreter/efun/mod.rs:214–226

Scope: `efuns` (macro). Role hint: `code`.

```
macro_rules! efuns {
    (@dispatch $name:ident, $context:ident) => {
        $name::$name($context)
    };
    (@dispatch $name:ident [in $module:ident], $context:ident) => {
        $module::$name($context)
    };
    (@dispatch $name:ident [async], $context:ident) => {
        $name::$name($context).await
    };
    (@dispatch $name:ident [async in $module:ident], $context:ident) => {
        $module::$name($context).await
    };

```

#### 2. src/compiler/codegen/efun_override_walker.rs:21–25

Scope: `ContextHolder for EfunOverrideWalker` (implementation). Role hint: `code`.

```
impl ContextHolder for EfunOverrideWalker {
    fn into_context(self) -> CompilationContext {
        self.context
    }
}
```

#### 3. lpc-rs-function-support/src/function_prototype.rs:85–87

Scope: `FunctionPrototype::is_efun` (function). Role hint: `code`.

```
pub fn is_efun(&self) -> bool {
        self.kind == FunctionKind::Efun
    }
```

#### 4. src/interpreter/efun/efun_context.rs:78–89

Scope: `EfunContext<'task, N>` (implementation). Role hint: `code`.

```
impl<'task, const N: usize> EfunContext<'task, N> {
    /// A context for `efun` with no arguments, at a call site.
    pub fn new(
        stack: &'task mut CallStack<N>,
        task_context: &'task TaskContext,
        efun: Efun,
    ) -> Self {
        Self::build(stack, task_context, efun, SmallVec::new(), None, None)
    }

    /// The context for `efun`, called by the top frame's instruction with
    /// `list`: the values read now, a by-reference cell's with them.

```

#### 5. lpc-rs-function-support/src/function_prototype.rs:34–51

Scope: `FunctionPrototype` (struct). Role hint: `code`.

```
pub struct FunctionPrototype {
    /// The name of the function
    #[builder(setter(into))]
    pub name: Cow<'static, str>,

    /// The filename where this function is defined. This is empty for efuns.
    #[builder(setter(into))]
    pub filename: Arc<LpcPath>,

    /// The return type
    pub return_type: LpcType,

    /// The kind of function this is
    #[builder(default)]
    pub kind: FunctionKind,

    /// The arity of the this function
    #[builder(default)]

```

#### 6. src/interpreter/efun/efun_context.rs:336–346

Scope: `EfunContext<'task, N>::return_efun_result` (function). Role hint: `code`.

```
pub fn return_efun_result(&mut self, result: LpcRef) {
        let frame = self.stack.last_mut().expect(NO_CALLING_FRAME);
        let slot = &mut frame.registers[0];
        match result {
            LpcRef::Int(value) => match slot {
                LpcRef::Int(x) => x.0 = value.0,
                _ => *slot = LpcRef::Int(value),
            },
            other => *slot = other,
        }
    }
```

#### 7. src/compiler/codegen/function_prototype_walker.rs:154–165

Scope: `TreeWalker for FunctionPrototypeWalker::visit_function_def` (function). Role hint: `code`.

```
            .context
            .lookup_function_complete(node.name, &CallNamespace::default());

        // An object's own definition outranks a nomask simul efun (CD, FluffOS).
        if let Some(Callee::Local(prototype)) = proto_opt
            && prototype.flags.nomask()
        {
            let e = LpcError::new(format!(
                "attempt to redefine nomask function `{}`",
                node.name
            ))
            .with_span(node.span)

```

#### 8. src/interpreter/task_context.rs:747–760

Scope: `TaskContext` (implementation). Role hint: `code`.

```
 pub fn simul_efuns(&self) -> Option<Arc<Process>> {
        if let Some(view) = &self.system_view {
            return view.simul.clone();
        }
        let path = self.config().simul_efun_source()?;
        if !self.txn.joinable() {
            return crate::util::get_simul_efuns(self.config(), self.object_space());
        }
        match self.find_object(&path) {
            ObjectLookup::Found(p) => Some(p),
            _ => None,
        }
    }


```

## show me the codepath that transaction commits take, with all the checks they go through

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| receiver | — | 1 | 1 |
| validator | — | 7 | 7 |
| future-version-check | — | 7 | 7 |

### compact source cards

#### 1. doc/transaction-town/README.md:1–4

Role hint: `guide`.

```
# Transaction Town

An offline, interactive explanation of transactions in the LPC driver. Owning task A takes a snapshot, reads cells, stages private changes and output, then submits to the single committer. Another task B can commit in the meantime. A rejected attempt takes the return road through retry admission and starts again; an accepted attempt delivers its receipt.


```

#### 2. doc/efun/get_char.md:27–32

Role hint: `guide`.

```

Registrations replace any pending `get_char` or `input_to` handler; the last
registration wins. They take effect in order with other socket operations
after the transaction commits. An aborted attempt registers nothing; conflict
retries publish only the successful attempt. A caught error preserves a
registration when its 
```

#### 3. doc/collector-town/js/world.js:27–28

Role hint: `code`.

```
      tag: 'Check snapshot pins', short: 'Even one held snapshot sends collection down the refusal road.',
      body: 'The committer sums live snapshot pins across all versions. With 0 pins it can trace and sweep safely. With even 1 pin it replies GcRefused immediately, before changing the world. It does not wait for that transaction or take a global lock. Try raising “Snapshot pins” and watch the cart turn left. The accepted road that follows expands one atomic committer operation for teac
```

#### 4. doc/command-rules.md:187–194

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

#### 5. doc/transaction-town/js/ui.js:352–365

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

#### 6. src/interpreter/process/util.rs:9–19

Scope: `AllEnvironment` (struct). Role hint: `code`.

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

#### 7. src/interpreter/sockets/transport.rs:1007–1020

Scope: `AttemptBody for Publication::commit_phase` (function). Role hint: `code`.

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

#### 8. src/interpreter/sockets/transport/tests.rs:49–55

Scope: `Fixture::commit` (function). Role hint: `code`.

```
async fn commit(&self, txn: &TxnHandle, mut live: LiveSnapshot) {
        live.disarm();
        commit_changeset(&self.gs.committer_tx, txn.with(Transaction::take_changeset))
            .await
            .unwrap()
            .unwrap();
    }
```

### search source cards

#### 1. src/interpreter/stm/committer.rs:279–288

Scope: `Committer::dispatch` (function). Role hint: `code`.

```
fn dispatch(&mut self, msg: CommitProtocol, tx: &flume::Sender<CommitProtocol>) -> bool {
        match msg {
            CommitProtocol::BootstrapObject(change) => {
                let mut changeset = Changeset::new(self.snapshot.version());
                changeset.object_changes.push(change);
                self.publish(changeset, false);
                if self.live_versions.is_empty() {
                    self.evict_old_versions();
                }
            }

```

#### 2. src/interpreter/stm/committer.rs:77–79

Scope: `CommitterStats::conflict` (function). Role hint: `code`.

```
fn conflict(&mut self) {
        self.conflicts += 1;
    }
```

#### 3. src/interpreter/sockets/callbacks.rs:272–284

Scope: `AttemptBody for Invocation<'_>::commit_phase` (function). Role hint: `code`.

```
async fn commit_phase(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
        _live: LiveSnapshot,
    ) -> Result<(std::result::Result<(), Conflict>, Vec<Effect>)> {
        let txn = self.txn.as_ref().expect("attempt opened");
        let changes = txn.with(|t| {
            if self.failure.is_some() {
                t.validation_changeset()
            } else {
                t.clone_changeset()
            }
        });

```

#### 4. src/interpreter/snooping/mod.rs:183–195

Scope: `Registration::valid` (function). Role hint: `code`.

```
pub(crate) fn valid(&self, txn: &mut Transaction) -> bool {
        let (Some(observer), Some(target), Some(connection)) = (
            self.observer.upgrade(),
            self.target.upgrade(),
            self.connection.upgrade(),
        ) else {
            return false;
        };
        observer.is_live_in(txn)
            && target.is_live_in(txn)
            && !connection.is_dead()
            && txn
                .read_connection(target.connection.id)

```

#### 5. src/interpreter/stm/committer.rs:242–244

Scope: `Committer::commit_watch` (function). Role hint: `code`.

```
pub(crate) fn commit_watch(&self) -> watch::Receiver<Version> {
        self.commit_watch.subscribe()
    }
```

#### 6. src/interpreter/stm/diagnostics.rs:34–56

Scope: `Conflict` (enum). Role hint: `code`.

```
pub(crate) enum Conflict {
    ReadInvalidated {
        cell: VarId,
        base: Version,
        current: Version,
        written_at: Version,
        writer: Option<Arc<CommitOrigin>>,
    },
    MergeMismatch {
        cell: VarId,
        base: Version,
        current: Version,
    },
    HistoryUnavailable {
        base: Version,
        oldest: Version,
        current: Version,
    },
    FutureVersion {
        base: Version,
        current: Version,
    },
    #[cfg(test)]

```

#### 7. src/interpreter/stm/committer.rs:611–619

Scope: `Committer::commit` (function). Role hint: `code`.

```
pub(crate) fn commit(&mut self, mut changeset: Changeset) -> Result<(), Conflict> {
        let current_version = self.snapshot.version();
        let changeset_version = changeset.base_version();

        // Should not occur in practice - it implies versions are being created in more than one place.
        if current_version < changeset_version {
            self.stats.conflict();
            return Err(Conflict::FutureVersion {
                base: changeset_version,

```

#### 8. doc/transaction-town/js/world.js:130–132

Role hint: `code`.

```
      short:
        'One committer checks dependencies and publishes accepted changes atomically.',
      body: 'For each commit newer than A’s base, the committer intersects the written cells with A’s read set. With the defaults, B wrote the 1 cell A read, so A is rejected and its buffers are discarded. If validation passes, writes apply in commit order and merges fold onto current values before publication. A blind write has no gold read to invalidate. No separate write-write check runs h
```
