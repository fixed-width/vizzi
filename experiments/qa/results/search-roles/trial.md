# Inspect source coverage for the maintainer questions

For the maintainer comparing deterministic source search. All source cards below come
from the pinned lpc-rs revision `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. These are locations and excerpts,
not generated answers or a resolved call graph. Scope and literal matches are recorded
separately in the assessment. The compact control shares the current role-specific index.

## where are new transactions created?

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| constructor | 3 | 1 | 1 |
| owning-entry | — | 5 | 5 |

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

#### 1. src/interpreter/stm/mod.rs:127–139

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

#### 2. src/interpreter/stm/mod.rs:385–395

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

#### 3. src/interpreter/sockets/transport.rs:940–951

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

#### 4. src/interpreter/stm/mod.rs:520–530

Scope: `Transaction::with_mapping_cow` (function). Role hint: `code`.

```
        }
        let current = self
            .read_value(var_id)
            .and_then(WorldValue::into_mapping)
            .unwrap_or_else(|| Arc::new(LpcMapping::default()));
        let mut clone = (*current).clone();
        f(&mut clone)?;
        self.changeset
            .write(var_id, WorldValue::Mapping(Arc::new(clone)));
        Ok(())
    }
```

#### 5. src/interpreter/task/mod.rs:381–398

Scope: `Task<STACKSIZE>::open_attempt` (function). Role hint: `code`.

```
async fn open_attempt(
        &mut self,
        tx: &flume::Sender<CommitProtocol>,
    ) -> Result<Option<LiveSnapshot>> {
        let live = if self.joins_parent {
            None
        } else {
            Some(start_txn(tx).await?)
        };

        if let Some(live) = &live {
            self.context.txn = TxnHandle::new(Transaction::new(live.inner.clone()));
            self.context.clear_commands();
        }

        let seed = self
            .seed
            .clone()

```

#### 6. src/interpreter/stm/mod.rs:560–562

Scope: `Transaction::pending_entry_view` (function). Role hint: `code`.

```
pub(crate) fn pending_entry_view(&self, server: &std::path::Path) -> PendingEntryView {
        PendingEntryView::new(&self.effects, server)
    }
```

#### 7. src/interpreter/stm/retry.rs:465–474

Scope: `attempt_timeout` (function). Role hint: `code`.

```
fn attempt_timeout(body: &impl AttemptBody, attempts: u64, conflicts: u64) -> LpcError {
    let error = body.timeout_error().with_note(format!(
        "Transaction aborted after {attempts} attempt(s) and {conflicts} conflict(s); uncommitted changes and output were discarded."
    ));
    if conflicts > 0 {
        error.with_note("Concurrent changes forced retries. Check shared state accessed by this operation and reduce the work done in one transaction.")
    } else {
        error
    }
}
```

#### 8. src/interpreter/stm/mod.rs:565–567

Scope: `Transaction::pending_directory_view` (function). Role hint: `code`.

```
pub(crate) fn pending_directory_view(&self, server: &std::path::Path) -> PendingDirectoryView {
        PendingDirectoryView::new(&self.effects, server)
    }
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

#### 2. src/compiler/codegen/efun_override_walker.rs:22–24

Scope: `ContextHolder for EfunOverrideWalker::into_context` (function). Role hint: `code`.

```
fn into_context(self) -> CompilationContext {
        self.context
    }
```

#### 3. src/interpreter/efun/mod.rs:1427–1429

Scope: `Efun::prototype` (function). Role hint: `code`.

```
pub fn prototype(self) -> &'static FunctionPrototype {
        &EFUN_PROTOTYPES[self as usize]
    }
```

#### 4. src/interpreter/efun/efun_context.rs:336–346

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

#### 5. src/interpreter/efun/function_signature.rs:10–18

Scope: `function_signature` (function). Role hint: `code`.

```
pub fn function_signature<const N: usize>(context: &mut EfunContext<'_, N>) -> Result<()> {
    let Some(name) = context.arg(0).as_str() else {
        return Err(context.runtime_error("function_signature: name must be a string"));
    };
    if context.arg(1) != &context.txn().nil() && !matches!(context.arg(1), LpcRef::Object(_)) {
        return Err(context.runtime_error("function_signature: target must be an object"));
    }
    let Some(function) = context
        .arg(1)

```

#### 6. src/util/process_builder.rs:115–128

Scope: `compile_process_in_context` (function). Role hint: `code`.

```
pub(crate) async fn compile_process_in_context(
    ctx: &crate::interpreter::task_context::TaskContext,
    path: &LpcPath,
    code: Option<&str>,
    gate: Arc<dyn CompileGate>,
    source_reader: Arc<dyn SourceReader>,
) -> Result<(Arc<Process>, Vec<LpcError>)> {
    let simul = ctx.simul_efuns().map(|process| process.program(ctx.txn()));
    compile_to_process(
        ctx.object_space(),
        Some(gate),
        Some(source_reader),
        simul,
        |compiler| async move {

```

#### 7. src/interpreter/efun/clone_object.rs:10–21

Scope: `load_prototype` (function). Role hint: `code`.

```
async fn load_prototype<const N: usize>(
    context: &mut EfunContext<'_, N>,
    path: &str,
) -> Result<Arc<Process>> {
    let full_path = context.in_game_path(path);

    if full_path.is_clone() {
        return Err(context.runtime_error(format!("Cannot clone a clone: {}", full_path)));
    }

    context.load_object(path).await
}
```

#### 8. src/interpreter/efun/efun_context.rs:580–582

Scope: `EfunContext<'task, N>::task_context` (function). Role hint: `code`.

```
pub fn task_context(&self) -> &TaskContext {
        self.task_context
    }
```

## show me the codepath that transaction commits take, with all the checks they go through

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| receiver | — | 1 | 1 |
| validator | — | 6 | 6 |
| future-version-check | — | 6 | 6 |

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

#### 6. src/interpreter/stm/committer.rs:611–619

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

#### 7. src/interpreter/stm/mod.rs:216–229

Scope: `Transaction::publish_system` (function). Role hint: `code`.

```
pub(crate) fn publish_system(
        &mut self,
        space: &Arc<ObjectSpace>,
        process: &Arc<Process>,
        insert: bool,
    ) {
        self.track_read(space.system_revision.id);
        self.write(space.system_revision.id, LpcRef::from(0));
        self.changeset
            .system_publications
            .push(changeset::SystemPublication {
                space: space.clone(),
                key: space.process_key(process),
                process: process.clone(),

```

#### 8. doc/transaction-town/js/world.js:130–132

Role hint: `code`.

```
      short:
        'One committer checks dependencies and publishes accepted changes atomically.',
      body: 'For each commit newer than A’s base, the committer intersects the written cells with A’s read set. With the defaults, B wrote the 1 cell A read, so A is rejected and its buffers are discarded. If validation passes, writes apply in commit order and merges fold onto current values before publication. A blind write has no gold read to invalidate. No separate write-write check runs h
```
