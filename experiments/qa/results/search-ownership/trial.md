# Inspect source coverage for the maintainer questions

For the maintainer comparing deterministic source search. All source cards below come
from the pinned lpc-rs revision `1a4f5b65dd2a269298748df2dfd68092a169a8d9`. These are locations and excerpts,
not generated answers or a resolved call graph. Scope and literal matches are recorded
separately in the assessment. The compact control shares the current ownership-policy index.

## where are new transactions created?

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| constructor | 3 | 1 | 1 |
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

#### 3. src/interpreter/stm/mod.rs:90–98

Scope: `Transaction` (struct). Role hint: `code`.

```
pub(crate) struct Transaction {
    snapshot: Snapshot,
    changeset: Changeset,
    /// Physical output recorded by this attempt, delivered once after a
    /// successful commit. Dropped with the attempt on conflict, so a
    /// re-run re-records it; never resolved from a cell at flush time.
    effects: Vec<Effect>,
    /// Call outs recorded by this attempt but not yet materialized. A
    /// separate list (not an `Effect`) because materialization needs the

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

#### 5. src/interpreter/stm/mod.rs:253–266

Scope: `Transaction::bootstrap_object_is_live` (function). Role hint: `code`.

```
        if !self.joinable {
            return true;
        }
        let filename = process.filename();
        let key = if filename.starts_with('/') {
            filename
        } else {
            ObjectSpace::with_leading_slash(&filename).into()
        };
        self.snapshot.objects.get(key.as_ref()).map_or_else(
            || process.cell.get().is_none(),
            |current| std::ptr::eq(current.as_ref(), process),
        )
    }
```

#### 6. src/interpreter/stm/mod.rs:560–562

Scope: `Transaction::pending_entry_view` (function). Role hint: `code`.

```
pub(crate) fn pending_entry_view(&self, server: &std::path::Path) -> PendingEntryView {
        PendingEntryView::new(&self.effects, server)
    }
```

#### 7. src/interpreter/stm/mod.rs:497–510

Scope: `Transaction::with_array_cow` (function). Role hint: `code`.

```
    ) -> Result<()> {
        if let Some(own) = self.changeset.written_array_mut(var_id) {
            return f(Arc::make_mut(own));
        }
        let current = self
            .read_value(var_id)
            .and_then(WorldValue::into_array)
            .unwrap_or_else(|| Arc::new(LpcArray::default()));
        let mut clone = (*current).clone();
        f(&mut clone)?;
        self.changeset
            .write(var_id, WorldValue::Array(Arc::new(clone)));
        Ok(())
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

#### 2. src/interpreter/efun/mod.rs:1403–1418

Scope: `Efun` (implementation). Role hint: `code`.

```
impl Efun {
    /// The efun at a
    /// [`CallEfun`](lpc_rs_asm::instruction::Instruction::CallEfun) index,
    /// `None` past the table.
    #[inline]
    pub fn from_index(index: usize) -> Option<Efun> {
        Self::ALL.get(index).copied()
    }

    /// The efun with this name, `None` for a name outside the table.
    pub fn from_name(name: &str) -> Option<Efun> {
        EFUN_PROTOTYPES
            .get_index_of(name)
            .and_then(Self::from_index)
    }


```

#### 3. src/interpreter/efun/mod.rs:1427–1429

Scope: `Efun::prototype` (function). Role hint: `code`.

```
pub fn prototype(self) -> &'static FunctionPrototype {
        &EFUN_PROTOTYPES[self as usize]
    }
```

#### 4. src/compiler/codegen/efun_override_walker.rs:21–25

Scope: `ContextHolder for EfunOverrideWalker` (implementation). Role hint: `code`.

```
impl ContextHolder for EfunOverrideWalker {
    fn into_context(self) -> CompilationContext {
        self.context
    }
}
```

#### 5. src/interpreter/efun/mod.rs:1433–1437

Scope: `Efun::name` (function). Role hint: `code`.

```
pub fn name(self) -> &'static str {
        EFUN_PROTOTYPES
            .get_index(self as usize)
            .map_or("", |(name, _)| name)
    }
```

#### 6. src/interpreter/efun/efun_context.rs:78–89

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

#### 7. lpc-rs-function-support/src/function_prototype.rs:85–87

Scope: `FunctionPrototype::is_efun` (function). Role hint: `code`.

```
pub fn is_efun(&self) -> bool {
        self.kind == FunctionKind::Efun
    }
```

#### 8. src/interpreter/efun/efun_context.rs:336–346

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

## show me the codepath that transaction commits take, with all the checks they go through

| Target | Compact rank | Search rank | Search literal rank |
| --- | ---: | ---: | ---: |
| receiver | — | — | — |
| validator | — | 1 | 1 |
| future-version-check | — | 1 | 1 |

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

#### 1. src/interpreter/stm/committer.rs:611–619

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

#### 2. src/interpreter/stm/mod.rs:91–98

Scope: `Transaction` (struct). Role hint: `code`.

```
    snapshot: Snapshot,
    changeset: Changeset,
    /// Physical output recorded by this attempt, delivered once after a
    /// successful commit. Dropped with the attempt on conflict, so a
    /// re-run re-records it; never resolved from a cell at flush time.
    effects: Vec<Effect>,
    /// Call outs recorded by this attempt but not yet materialized. A
    /// separate list (not an `Effect`) because materialization needs the

```

#### 3. src/interpreter/stm/mod.rs:216–229

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

#### 4. src/interpreter/stm/committer.rs:77–79

Scope: `CommitterStats::conflict` (function). Role hint: `code`.

```
fn conflict(&mut self) {
        self.conflicts += 1;
    }
```

#### 5. src/interpreter/stm/mod.rs:433–445

Scope: `Transaction::publish_socket` (function). Role hint: `code`.

```
pub(crate) fn publish_socket(
        &mut self,
        cell: VarId,
        value: Option<crate::interpreter::sockets::Value>,
        sender: &flume::Sender<crate::interpreter::sockets::SocketCommands>,
        command: crate::interpreter::sockets::SocketCommand,
    ) {
        match value {
            Some(value) => self.write_socket(cell, value),
            None => self.drop_var(cell),
        }
        if let Some(publication) = self
            .changeset

```

#### 6. src/interpreter/snooping/mod.rs:183–195

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

#### 7. src/interpreter/stm/mod.rs:145–147

Scope: `Transaction::set_origin` (function). Role hint: `code`.

```
pub(crate) fn set_origin(&mut self, process: &Process, entry: &str) {
        self.changeset.origin = Some(Arc::new(CommitOrigin::new(process, entry)));
    }
```

#### 8. src/interpreter/stm/diagnostics.rs:34–56

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
