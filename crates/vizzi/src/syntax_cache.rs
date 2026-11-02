//! Shared retained-syntax admission with exclusive, nonblocking leases.

use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Mutex, MutexGuard},
};
use vizzi_core::{
    analysis::SyntaxState,
    model::{ContentId, Context, ContextId, ModelError, Snapshot, WorkspaceId},
};

/// Workspace/path and complete descriptor; content belongs to the owned state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyntaxKey {
    workspace: WorkspaceId,
    path: String,
    context: ContextId,
    language: String,
    description: String,
}
impl SyntaxKey {
    /// Uses a normalised captured path, independently of source control or timestamps.
    pub fn new(workspace: &WorkspaceId, path: &str, context: &Context) -> Self {
        Self {
            workspace: workspace.clone(),
            path: path.into(),
            context: context.id.clone(),
            language: context.language.clone(),
            description: context.description.clone(),
        }
    }
    fn matches(&self, context: &Context) -> bool {
        self.context == context.id
            && self.language == context.language
            && self.description == context.description
    }
}

/// Cumulative cache observations; weights are not native heap measurements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SyntaxCacheStats {
    /// Configured aggregate admission weight.
    pub budget_weight: usize,
    /// Largest new state allowance per lease.
    pub entry_weight_limit: usize,
    /// Idle state plus active old/new reservations.
    pub charged_weight: usize,
    /// Highest observed aggregate charge.
    pub peak_charged_weight: usize,
    /// Retained entries available for checkout.
    pub idle_entries: usize,
    /// Source bytes in idle entries; leased state is covered by reservations.
    pub idle_source_bytes: usize,
    /// Descendant nodes in idle trees.
    pub idle_nodes: usize,
    /// Exclusively checked-out keys, including invalidated reservations.
    pub active_leases: usize,
    /// Idle entries discarded to satisfy admission limits.
    pub evicted: usize,
    /// Requested leases bypassed because a key or quota was unavailable.
    pub bypassed: usize,
}

/// Point observation of one key, without transferring or touching its retained state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SyntaxEntry {
    /// Exclusively checked out, including an invalidated reservation awaiting release.
    Leased,
    /// Immutable accounting for state available to the next lease.
    Idle {
        /// Admission weight of the retained source and tree.
        weight: usize,
        /// Exact retained source byte count.
        source_bytes: usize,
        /// Descendant nodes in the retained tree.
        nodes: usize,
    },
}

struct Inner {
    idle: VecDeque<(SyntaxKey, SyntaxState)>,
    active: BTreeMap<SyntaxKey, bool>,
    charged: usize,
    peak: usize,
    evicted: usize,
    bypassed: usize,
}
/// One cache may be shared by multiple engines; all languages use the same quota.
pub struct SyntaxCache {
    budget: usize,
    entry: usize,
    count: usize,
    inner: Mutex<Inner>,
}
impl SyntaxCache {
    /// Host policy: up to eight entries, with old/new overlap and a 256 MiB entry cap.
    pub fn for_budget(budget: usize) -> Result<Self, ModelError> {
        Self::new(budget, (budget / 2).min(256 * 1024 * 1024), 8)
    }
    /// Configures positive weights/count; two entry allowances must fit the total.
    pub fn new(budget: usize, entry: usize, count: usize) -> Result<Self, ModelError> {
        if budget == 0 || entry == 0 || entry > budget / 2 || count == 0 {
            return Err(ModelError(
                "syntax retention requires 0 < entry <= budget/2 and a positive entry count".into(),
            ));
        }
        Ok(Self {
            budget,
            entry,
            count,
            inner: Mutex::new(Inner {
                idle: VecDeque::new(),
                active: BTreeMap::new(),
                charged: 0,
                peak: 0,
                evicted: 0,
                bypassed: 0,
            }),
        })
    }
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Observes a complete key without leasing state, promoting it, or changing LRU order.
    /// Absence returns `None`; another thread can change availability after this call.
    pub fn entry(&self, key: &SyntaxKey) -> Option<SyntaxEntry> {
        let inner = self.lock();
        if inner.active.contains_key(key) {
            return Some(SyntaxEntry::Leased);
        }
        inner
            .idle
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, state)| SyntaxEntry::Idle {
                weight: state.weight(),
                source_bytes: state.source_bytes(),
                nodes: state.nodes(),
            })
    }
    /// Exclusively checks out compatible state; promotion allows a new key.
    /// Unavailable admission returns immediately without waiting for another lease.
    pub fn lease(&self, key: SyntaxKey, promote: bool) -> Option<SyntaxLease<'_>> {
        let mut inner = self.lock();
        if inner.active.contains_key(&key) {
            inner.bypassed += 1;
            return None;
        }
        let position = inner
            .idle
            .iter()
            .position(|(candidate, _)| candidate == &key);
        if position.is_none() && !promote {
            return None;
        }
        let previous = position
            .and_then(|index| inner.idle.remove(index))
            .map(|(_, state)| state);
        let old_weight = previous.as_ref().map_or(0, SyntaxState::weight);
        let evictable: usize = inner.idle.iter().map(|(_, state)| state.weight()).sum();
        if inner.active.len() >= self.count || inner.charged - evictable > self.budget - self.entry
        {
            if let Some(state) = previous {
                inner.idle.push_back((key, state));
            }
            inner.bypassed += 1;
            return None;
        }
        let mut evicted = Vec::new();
        while inner.charged > self.budget - self.entry
            || inner.idle.len() + inner.active.len() >= self.count
        {
            if let Some((_, state)) = inner.idle.pop_front() {
                inner.charged -= state.weight();
                inner.evicted += 1;
                evicted.push(state);
            }
        }
        inner.charged += self.entry;
        inner.peak = inner.peak.max(inner.charged);
        inner.active.insert(key.clone(), true);
        drop(inner);
        drop(evicted);
        Some(SyntaxLease {
            cache: self,
            key,
            previous,
            reserved: old_weight + self.entry,
            finished: false,
        })
    }
    /// Invalidates idle and active entries for a known removed or reassigned path.
    pub fn invalidate(&self, workspace: &WorkspaceId, path: &str) {
        self.invalidate_where(|key| &key.workspace == workspace && key.path == path);
    }
    pub(crate) fn reconcile(&self, snapshot: &Snapshot) {
        self.invalidate_where(|key| {
            &key.workspace == snapshot.workspace()
                && !snapshot.file_by_path(&key.path).is_some_and(|file| {
                    file.analyses()
                        .iter()
                        .any(|analysis| key.matches(&analysis.context))
                })
        });
    }
    fn invalidate_where(&self, invalid: impl Fn(&SyntaxKey) -> bool) {
        let mut inner = self.lock();
        let mut discarded = Vec::new();
        let mut index = 0;
        while index < inner.idle.len() {
            if invalid(&inner.idle[index].0) {
                if let Some((_, state)) = inner.idle.remove(index) {
                    inner.charged -= state.weight();
                    discarded.push(state);
                }
            } else {
                index += 1;
            }
        }
        for (key, valid) in &mut inner.active {
            if invalid(key) {
                *valid = false;
            }
        }
        drop(inner);
        drop(discarded);
    }
    /// Reads aggregate accounting without exposing owned tree state.
    pub fn stats(&self) -> SyntaxCacheStats {
        let inner = self.lock();
        SyntaxCacheStats {
            budget_weight: self.budget,
            entry_weight_limit: self.entry,
            charged_weight: inner.charged,
            peak_charged_weight: inner.peak,
            idle_entries: inner.idle.len(),
            idle_source_bytes: inner
                .idle
                .iter()
                .map(|(_, state)| state.source_bytes())
                .sum(),
            idle_nodes: inner.idle.iter().map(|(_, state)| state.nodes()).sum(),
            active_leases: inner.active.len(),
            evicted: inner.evicted,
            bypassed: inner.bypassed,
        }
    }
}

/// A reservation releases on drop, including analyzer failure or cancellation.
pub struct SyntaxLease<'a> {
    cache: &'a SyntaxCache,
    key: SyntaxKey,
    previous: Option<SyntaxState>,
    reserved: usize,
    finished: bool,
}
impl SyntaxLease<'_> {
    /// Moves the old state into exactly one analyzer invocation.
    pub fn take_previous(&mut self) -> Option<SyntaxState> {
        self.previous.take()
    }
    /// Maximum admission weight for returned new state.
    pub fn allowance(&self) -> usize {
        self.cache.entry
    }
    /// Admits only matching complete state, and releases the old/new reservation.
    pub fn finish(mut self, mut state: Option<SyntaxState>, expected: &ContentId) -> bool {
        let mut inner = self.cache.lock();
        let valid = inner.active.remove(&self.key) == Some(true);
        inner.charged -= self.reserved;
        let accepted = valid
            && state.as_ref().is_some_and(|state| {
                state.weight() <= self.cache.entry
                    && self.key.matches(state.context())
                    && state.content() == expected
            });
        if accepted && let Some(state) = state.take() {
            inner.charged += state.weight();
            inner.idle.push_back((self.key.clone(), state));
        }
        self.finished = true;
        drop(inner);
        drop(state);
        accepted
    }
}
impl Drop for SyntaxLease<'_> {
    fn drop(&mut self) {
        if !self.finished {
            let mut inner = self.cache.lock();
            inner.active.remove(&self.key);
            inner.charged -= self.reserved;
        }
    }
}
