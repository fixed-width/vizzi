//! Mixed-language scheduling, configuration, and adapter failure contracts.

use std::{
    path::Path,
    sync::{
        Arc, Barrier,
        atomic::{AtomicUsize, Ordering},
    },
};
use vizzi::{
    cache::Cache,
    engine::Engine,
    source::{CaptureScope, Directory},
};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    model::{Analysis, Context, ContextId, ModelError},
    python::PythonAdapter,
    rust::RustAdapter,
};

#[derive(Default)]
struct Activity {
    active: AtomicUsize,
    peak: AtomicUsize,
    started: AtomicUsize,
    created: AtomicUsize,
    rendezvous: Option<(usize, Barrier)>,
}
#[derive(Clone, Copy, Default)]
enum Fault {
    #[default]
    None,
    Create,
    Analyze,
    Context,
    Content,
}
struct Adapter {
    inner: Arc<dyn LanguageAdapter>,
    activity: Arc<Activity>,
    revision: u8,
    fault: Fault,
}
impl LanguageAdapter for Adapter {
    fn context(&self) -> Context {
        let mut context = self.inner.context();
        context.id = ContextId::derive(&[context.id.as_str().as_bytes(), &[self.revision]]);
        context
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        if matches!(self.fault, Fault::Create) {
            return Err(ModelError("factory failed".into()));
        }
        self.activity.created.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Parser {
            inner: self.inner.create()?,
            activity: self.activity.clone(),
            context: self.context(),
            fault: self.fault,
        }))
    }
}
struct Parser {
    inner: Box<dyn Analyzer>,
    activity: Arc<Activity>,
    context: Context,
    fault: Fault,
}
impl Analyzer for Parser {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        let active = self.activity.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.activity.peak.fetch_max(active, Ordering::SeqCst);
        let started = self.activity.started.fetch_add(1, Ordering::SeqCst);
        if let Some((width, barrier)) = &self.activity.rendezvous
            && started < *width
        {
            barrier.wait();
        }
        let mut result = self.inner.analyze(bytes)?;
        self.activity.active.fetch_sub(1, Ordering::SeqCst);
        result.context = self.context.clone();
        match self.fault {
            Fault::Analyze => return Err(ModelError("analysis failed".into())),
            Fault::Context => result.context.description = "unexpected context".into(),
            Fault::Content => {
                result.content = vizzi_core::model::ContentId::derive(&[b"wrong bytes"])
            }
            _ => {}
        }
        Ok(result)
    }
}
fn configured(
    root: &Path,
    cache: &Path,
    workers: usize,
    activity: &Arc<Activity>,
    revision: u8,
    fault: Fault,
    python: bool,
) -> Engine {
    let mut adapters: Vec<Arc<dyn LanguageAdapter>> = vec![Arc::new(Adapter {
        inner: Arc::new(RustAdapter),
        activity: activity.clone(),
        revision,
        fault,
    })];
    if python {
        adapters.push(Arc::new(Adapter {
            inner: Arc::new(PythonAdapter),
            activity: activity.clone(),
            revision,
            fault,
        }));
    }
    Engine::new(
        Directory::with_languages(root, Arc::new(Languages::new(adapters).unwrap())).unwrap(),
        Cache::new(cache.into()),
        workers,
    )
    .unwrap()
}

#[test]
fn one_budget_bounds_both_languages_and_warm_hits_create_no_parsers() {
    let root = tempfile::tempdir().unwrap();
    for index in 0..16 {
        std::fs::write(
            root.path().join(format!("{index:02}.rs")),
            format!("fn rust{index}() {{}}"),
        )
        .unwrap();
        std::fs::write(
            root.path().join(format!("{index:02}.py")),
            format!("def python{index}(): pass\n"),
        )
        .unwrap();
    }
    let mut expected = None;
    for workers in [1, 2, 4] {
        let cache = tempfile::tempdir().unwrap();
        let activity = Arc::new(Activity {
            rendezvous: Some((workers, Barrier::new(workers))),
            ..Default::default()
        });
        let engine = configured(
            root.path(),
            cache.path(),
            workers,
            &activity,
            0,
            Fault::None,
            true,
        );
        let cold = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(cold.measurements.parsed, 32);
        assert_eq!(activity.peak.load(Ordering::SeqCst), workers);
        assert_eq!(activity.active.load(Ordering::SeqCst), 0);
        let created = activity.created.load(Ordering::SeqCst);
        assert!((2..=workers * 2).contains(&created));
        if let Some(id) = &expected {
            assert_eq!(cold.snapshot.id(), id);
        }
        expected = Some(cold.snapshot.id().clone());
        let warm = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(warm.measurements.cache_hits, 32);
        assert_eq!(activity.created.load(Ordering::SeqCst), created);
        assert_eq!(activity.started.load(Ordering::SeqCst), 32);
        assert_eq!(cold.snapshot.id(), warm.snapshot.id());
    }
}

#[test]
fn configuration_changes_reconcile_unread_documents_and_invalidate_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn rust() {}").unwrap();
    std::fs::write(root.path().join("b.py"), "def python(): pass\n").unwrap();
    let activity = Arc::new(Activity::default());
    let original = configured(
        root.path(),
        cache.path(),
        2,
        &activity,
        0,
        Fault::None,
        true,
    )
    .refresh(None, &|| false)
    .unwrap()
    .unwrap();
    let scope = CaptureScope::Paths(["a.rs".into()].into());
    let changed_engine = configured(
        root.path(),
        cache.path(),
        2,
        &activity,
        1,
        Fault::None,
        true,
    );
    let changed = changed_engine
        .refresh_scoped(Some(&original.snapshot), &scope, &|| false)
        .unwrap()
        .unwrap();
    assert!(changed.measurements.reconciled);
    assert_eq!(changed.measurements.read_files, 2);
    assert_eq!(changed.measurements.parsed, 2);
    assert_eq!(changed.measurements.retained, 0);
    assert_ne!(original.snapshot.id(), changed.snapshot.id());
    assert_eq!(
        changed.snapshot.id(),
        changed_engine
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
    let rust_only = configured(
        root.path(),
        cache.path(),
        2,
        &activity,
        1,
        Fault::None,
        false,
    );
    let reduced = rust_only
        .refresh_scoped(Some(&changed.snapshot), &scope, &|| false)
        .unwrap()
        .unwrap();
    assert!(reduced.measurements.reconciled);
    assert_eq!(reduced.measurements.files, 2);
    assert_eq!(reduced.measurements.analyzed, 1);
    assert_eq!(reduced.measurements.source_only, 1);
    assert!(matches!(
        reduced.snapshot.documents()[1].classification(),
        vizzi_core::policy::Classification::SourceOnly {
            reason: vizzi_core::policy::SourceOnlyReason::AnalyzerUnavailable,
            ..
        }
    ));
    assert_eq!(reduced.snapshot.documents()[0].path(), "a.rs");
    assert_eq!(
        reduced.snapshot.id(),
        rust_only
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn invalid_adapter_output_and_errors_do_not_poison_the_cache() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn rust() {}").unwrap();
    let activity = Arc::new(Activity::default());
    for fault in [
        Fault::Create,
        Fault::Analyze,
        Fault::Context,
        Fault::Content,
    ] {
        let cache = tempfile::tempdir().unwrap();
        let bad = configured(root.path(), cache.path(), 1, &activity, 0, fault, false);
        assert!(bad.refresh(None, &|| false).is_err());
        let good = configured(
            root.path(),
            cache.path(),
            1,
            &activity,
            0,
            Fault::None,
            false,
        )
        .refresh(None, &|| false)
        .unwrap()
        .unwrap();
        assert_eq!(good.measurements.parsed, 1);
        assert_eq!(good.measurements.cache_hits, 0);
        assert_eq!(
            good.snapshot.documents()[0].analyses()[0].facts[0].name,
            "rust"
        );
    }
}

#[test]
fn cached_context_metadata_must_match_the_registered_descriptor() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.py"), "def python(): pass\n").unwrap();
    let languages = Languages::builtin().unwrap();
    let language = languages.select("python").unwrap();
    let mut analysis = language
        .create()
        .unwrap()
        .analyze(b"def python(): pass\n")
        .unwrap();
    analysis.context.description = "stale metadata with the same context id".into();
    Cache::new(cache.path().into()).store(&analysis).unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    let refreshed = engine.refresh(None, &|| false).unwrap().unwrap();
    assert_eq!(refreshed.measurements.parsed, 1);
    assert_eq!(refreshed.measurements.cache_hits, 0);
    assert_eq!(
        refreshed.snapshot.documents()[0].analyses()[0].context,
        *language.context()
    );
}
