//! Shared worker-local parser/cache execution for live and held review inputs.
use crate::{
    admission::{Admission, Priority, Refusal},
    cache::Cache,
    engine::{EngineError, Measurements},
    source::CapturedFile,
    syntax_cache::{SyntaxCache, SyntaxKey},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use vizzi_core::{
    analysis::{AnalysisResult, Analyzer, Language},
    model::{Analysis, Coverage, Document, WorkspaceId},
};

pub(crate) struct JobResults {
    pub documents: Vec<Arc<Document>>,
    pub measurements: Measurements,
}
pub(crate) struct Jobs<'a> {
    pub workspace: &'a WorkspaceId,
    pub cache: &'a Cache,
    pub syntax_cache: Option<&'a Arc<SyntaxCache>>,
    pub admission: Option<&'a Arc<Admission>>,
    pub incremental: bool,
}
impl Jobs<'_> {
    pub(crate) fn run(
        &self,
        jobs: &[(CapturedFile, &Language)],
        workers: usize,
        priority: Priority,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Option<JobResults>, EngineError> {
        let mut documents = vec![];
        let mut measurements = Measurements::default();
        let next = AtomicUsize::new(0);
        let batches = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers.min(jobs.len()))
                .map(|_| {
                    scope.spawn(|| -> Result<_, EngineError> {
                        let mut parsers = BTreeMap::<_, Box<dyn Analyzer>>::new();
                        let mut batch = Vec::new();
                        let mut counters = (0, 0, 0, 0);
                        loop {
                            if cancelled() {
                                break;
                            }
                            let index = next.fetch_add(1, Ordering::Relaxed);
                            let Some((file, language)) = jobs.get(index) else {
                                break;
                            };
                            let context = language.context();
                            let content = file.source.content();
                            let _permit = match self.admission.as_ref().map(|budget| budget.acquire(file.source.len(), priority)) {
                                Some(Ok(permit)) => Some(permit),
                                Some(Err(reason)) => {
                                    let diagnostic = match reason {
                                        Refusal::Busy => "Analysis deferred: active input capacity is occupied or reserved for foreground work; source remains available".into(),
                                        Refusal::FileLimit => format!("Analysis deferred: {} input bytes exceed the per-file admission limit; source remains available", file.source.len()),
                                    };
                                    batch.push(Arc::new(Document::capture_source(
                                        self.workspace.clone(), file.path.clone(), file.source.clone(),
                                        vec![Analysis { calls: None, coloring: None,
content: content.clone(), context: context.clone(), coverage: Coverage::Deferred, facts: vec![], diagnostics: vec![diagnostic] }],
                                        file.classification.clone(),
                                    )?));
                                    continue;
                                }
                                None => None,
                            };
                            let input=file.source.read_all(file.source.len())?;
                            let cached = self
                                .cache
                                .load(&context.id, content)
                                .filter(|analysis| analysis.context == *context)
                                .and_then(|analysis| {
                                    Document::capture_loaded(
                                        self.workspace.clone(),
                                        file.path.clone(),
                                        file.source.clone(),
                                        &input,
                                        vec![analysis],
                                        file.classification.clone(),
                                    )
                                    .ok()
                                });
                            let document = if let Some(document) = cached {
                                counters.1 += 1;
                                document
                            } else {
                                let parser = match parsers.entry(context.id.clone()) {
                                    std::collections::btree_map::Entry::Occupied(entry) => {
                                        entry.into_mut()
                                    }
                                    std::collections::btree_map::Entry::Vacant(entry) => {
                                        entry.insert(language.create()?)
                                    }
                                };
                                let mut lease = self.syntax_cache.as_ref().and_then(|cache| {
                                    cache.lease(
                                        SyntaxKey::new(
                                            self.workspace,
                                            &file.path,
                                            context,
                                        ),
                                        self.incremental,
                                    )
                                });
                                let result = if let Some(lease) = &mut lease {
                                    parser.analyze_with_state(
                                        input.bytes(),
                                        lease.take_previous(),
                                        lease.allowance(),
                                        cancelled,
                                    )?
                                } else {
                                    parser
                                        .analyze_cancellable(input.bytes(), cancelled)?
                                        .map(|analysis| AnalysisResult {
                                            analysis,
                                            state: None,
                                            tree_reused: false,
                                        })
                                };
                                let Some(result) = result else {
                                    return Ok(None);
                                };
                                let analysis = result.analysis;
                                if analysis.context != *context {
                                    return Err(EngineError::Configuration(
                                        "analyzer returned an unregistered context".into(),
                                    ));
                                }
                                let document = Document::capture_loaded(
                                    self.workspace.clone(),
                                    file.path.clone(),
                                    file.source.clone(),
                                    &input,
                                    vec![analysis],
                                    file.classification.clone(),
                                )?;
                                if let Some(lease) = lease {
                                    lease.finish(result.state, content);
                                }
                                counters.0 += 1;
                                counters.3 += usize::from(result.tree_reused);
                                if self.cache.store(&document.analyses()[0]).is_err() {
                                    counters.2 += 1;
                                }
                                document
                            };
                            batch.push(Arc::new(document));
                        }
                        Ok(Some((batch, counters)))
                    })
                })
                .collect();
            // Join every worker before propagating errors so multiple panics stay contained.
            handles
                .into_iter()
                .map(|handle| handle.join())
                .collect::<Vec<_>>()
                .into_iter()
                .map(|result| {
                    result.map_err(|_| {
                        EngineError::Configuration("analysis worker panicked".into())
                    })?
                })
                .collect::<Result<Vec<_>, EngineError>>()
        })?;
        if cancelled() {
            return Ok(None);
        }
        for batch in batches {
            let Some((batch, (parsed, hits, warnings, tree_reuses))) = batch else {
                return Ok(None);
            };
            documents.extend(batch);
            measurements.parsed += parsed;
            measurements.cache_hits += hits;
            measurements.cache_warnings += warnings;
            measurements.tree_reuses += tree_reuses;
        }
        Ok(Some(JobResults {
            documents,
            measurements,
        }))
    }
}
