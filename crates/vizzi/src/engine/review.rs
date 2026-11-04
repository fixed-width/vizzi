//! Rebind admitted immutable source to the engine's frozen analysis plan.

use super::*;
use crate::source::CapturedFile;
use vizzi_core::{
    model::{CapturePlanId, CapturePolicyId},
    policy::{Classification, Provenance, SourceOnlyReason},
};

impl Engine {
    /// Returns aggregate live/review/recovery parser allocation counters.
    pub fn analysis_workers(&self) -> WorkerStats {
        self.analysis_workers.stats()
    }

    pub(crate) fn review_plan(&self) -> &CapturePlanId {
        self.directory.plan().id()
    }

    pub(crate) fn analyze_review(
        &self,
        source: &Snapshot,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<Refresh>, EngineError> {
        if source.workspace() != self.directory.workspace() {
            return Err(EngineError::Configuration(
                "review source belongs to another logical workspace".into(),
            ));
        }
        let started = Instant::now();
        let mut documents = vec![];
        let mut jobs = vec![];
        for document in source.documents() {
            if cancelled() {
                return Ok(None);
            }
            let classification = self
                .directory
                .policy()
                .classify(document.path())
                .ok()
                .flatten()
                .unwrap_or(Classification::SourceOnly {
                    language: None,
                    provenance: Provenance::ExplicitScope { scope: ".".into() },
                    reason: SourceOnlyReason::OutsideAnalysisScope,
                });
            if let Some(language) = self.directory.language(&classification) {
                jobs.push((
                    CapturedFile {
                        path: document.path().to_owned(),
                        source: document.captured().clone(),
                        classification,
                    },
                    language,
                ));
            } else {
                documents.push(Arc::new(Document::capture_source(
                    source.workspace().clone(),
                    document.path().to_owned(),
                    document.captured().clone(),
                    vec![],
                    classification,
                )?));
            }
        }
        jobs.sort_by_key(|job| std::cmp::Reverse(job.0.source.len()));
        let Some(allocation) = self.analysis_workers.acquire(
            usize::from(!jobs.is_empty()),
            Priority::Recovery,
            cancelled,
        ) else {
            return Ok(None);
        };
        let Some(result) = (crate::analysis_jobs::Jobs {
            workspace: source.workspace(),
            cache: &self.cache,
            syntax_cache: None,
            admission: self.admission.as_ref(),
            incremental: false,
        })
        .run(&jobs, allocation.workers, Priority::Recovery, cancelled)?
        else {
            return Ok(None);
        };
        documents.extend(result.documents);
        drop(allocation);
        if cancelled() {
            return Ok(None);
        }
        let policy = CapturePolicyId::derive(&[
            b"review-analysis-v1",
            self.review_plan().as_str().as_bytes(),
        ]);
        let snapshot = Arc::new(Snapshot::new(
            source.workspace().clone(),
            documents,
            policy,
        )?);
        let mut measurements = result.measurements;
        measurements.files = snapshot.documents().len();
        measurements.analyzed = snapshot.analyzed_files();
        measurements.deferred = snapshot.deferred_files();
        measurements.source_only = measurements.files - measurements.analyzed;
        measurements.bytes = source
            .documents()
            .iter()
            .map(|document| document.bytes())
            .sum();
        measurements.retained = source.documents().len();
        measurements.total_ms = milliseconds(started.elapsed());
        Ok(Some(Refresh {
            snapshot,
            directories: None,
            measurements,
        }))
    }
}
