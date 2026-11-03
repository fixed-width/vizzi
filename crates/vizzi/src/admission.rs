//! Shared, nonblocking admission for active analysis input volume.

use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use vizzi_core::model::ModelError;

/// Foreground work can use capacity reserved from periodic recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    /// Explicit or event-driven indexing.
    Foreground,
    /// Private periodic verification.
    Recovery,
}
/// Reason an analysis input was not admitted in this attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// Input exceeds the configured per-file byte limit.
    FileLimit,
    /// Other active work or foreground headroom leaves insufficient capacity.
    Busy,
}
/// Input accounting, distinct from parser heap or whole-process memory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AdmissionStats {
    /// Aggregate admitted input-byte capacity.
    pub capacity_bytes: usize,
    /// Maximum captured input size for one job.
    pub file_limit_bytes: usize,
    /// Capacity withheld from recovery work.
    pub foreground_reserve_bytes: usize,
    /// Current charged input bytes; empty inputs charge one byte.
    pub active_bytes: usize,
    /// Current recovery input charges.
    pub recovery_bytes: usize,
    /// Current admitted job count.
    pub active_jobs: usize,
    /// Largest aggregate charge observed.
    pub peak_bytes: usize,
    /// Inputs refused because of size.
    pub oversized: usize,
    /// Inputs refused because capacity was occupied or reserved.
    pub busy: usize,
}
#[derive(Default)]
struct Active {
    bytes: usize,
    recovery: usize,
    jobs: usize,
    peak: usize,
    oversized: usize,
    busy: usize,
}
/// One admission object can be shared across languages, attempts and engines.
pub struct Admission {
    capacity: usize,
    file: usize,
    reserve: usize,
    active: Mutex<Active>,
}
impl Admission {
    /// Reserves one quarter for foreground; any admitted file fits the recovery share.
    pub fn for_budget(bytes: usize) -> Result<Self, ModelError> {
        let reserve = bytes / 4;
        Self::new(bytes, bytes - reserve, reserve)
    }
    /// Requires positive capacity/file limit and a per-file limit within recovery capacity.
    pub fn new(capacity: usize, file: usize, reserve: usize) -> Result<Self, ModelError> {
        if capacity == 0 || file == 0 || reserve >= capacity || file > capacity - reserve {
            return Err(ModelError(
                "analysis admission requires 0 < file limit <= capacity - foreground reserve"
                    .into(),
            ));
        }
        Ok(Self {
            capacity,
            file,
            reserve,
            active: Mutex::new(Active::default()),
        })
    }
    fn lock(&self) -> MutexGuard<'_, Active> {
        self.active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    /// Attempts admission immediately, without queueing or waiting for active analysis.
    pub fn acquire(&self, bytes: usize, priority: Priority) -> Result<Permit<'_>, Refusal> {
        let weight = bytes.max(1);
        let mut active = self.lock();
        if weight > self.file {
            active.oversized += 1;
            return Err(Refusal::FileLimit);
        }
        if weight > self.capacity - active.bytes
            || (priority == Priority::Recovery
                && weight > self.capacity - self.reserve - active.recovery)
        {
            active.busy += 1;
            return Err(Refusal::Busy);
        }
        active.bytes += weight;
        active.jobs += 1;
        if priority == Priority::Recovery {
            active.recovery += weight;
        }
        active.peak = active.peak.max(active.bytes);
        Ok(Permit {
            admission: self,
            weight,
            priority,
        })
    }
    /// Observes shared accounting without reading source or running an analyser.
    pub fn stats(&self) -> AdmissionStats {
        let active = self.lock();
        AdmissionStats {
            capacity_bytes: self.capacity,
            file_limit_bytes: self.file,
            foreground_reserve_bytes: self.reserve,
            active_bytes: active.bytes,
            recovery_bytes: active.recovery,
            active_jobs: active.jobs,
            peak_bytes: active.peak,
            oversized: active.oversized,
            busy: active.busy,
        }
    }
}
/// A job's input charge, released on completion, cancellation, error or unwinding.
pub struct Permit<'a> {
    admission: &'a Admission,
    weight: usize,
    priority: Priority,
}
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut active = self.admission.lock();
        active.bytes -= self.weight;
        active.jobs -= 1;
        if self.priority == Priority::Recovery {
            active.recovery -= self.weight;
        }
    }
}
