//! Interrupted adapters cannot publish or persist incomplete analyses.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use vizzi::{application::Application, cache::Cache, engine::Engine, source::Directory};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    model::{Analysis, Context, ModelError},
    rust::RustAdapter,
};

struct Adapter {
    stop: Arc<AtomicBool>,
    request: bool,
}
impl LanguageAdapter for Adapter {
    fn context(&self) -> Context {
        RustAdapter.context()
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(Parser {
            inner: RustAdapter.create()?,
            stop: self.stop.clone(),
            request: self.request,
        }))
    }
}
struct Parser {
    inner: Box<dyn Analyzer>,
    stop: Arc<AtomicBool>,
    request: bool,
}
impl Analyzer for Parser {
    fn analyze(&mut self, _: &[u8]) -> Result<Analysis, ModelError> {
        panic!("engine must call the cancellable operation")
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        if self.request {
            self.stop.store(true, Ordering::Release);
            assert!(cancelled());
        }
        if self.stop.load(Ordering::Acquire) {
            return Ok(None);
        }
        self.inner.analyze_cancellable(bytes, cancelled)
    }
}

#[test]
fn interrupted_attempt_keeps_pinned_source_and_does_not_cache_the_cancelled_file() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let engine = |request| {
        Engine::new(
            Directory::with_languages(
                root.path(),
                Arc::new(
                    Languages::new(vec![Arc::new(Adapter {
                        stop: stop.clone(),
                        request,
                    })])
                    .unwrap(),
                ),
            )
            .unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap()
    };
    let source = root.path().join("a.rs");
    std::fs::write(&source, "fn original() {}").unwrap();
    let app = Application::new("cancel fixture".into()).unwrap();
    let ticket = app.request();
    let original = engine(false).refresh(None, &|| false).unwrap().unwrap();
    let old = original.snapshot.clone();
    app.publish(ticket, original);
    std::fs::write(&source, "fn changed() {}").unwrap();
    assert!(
        engine(true)
            .refresh(Some(&old), &|| stop.load(Ordering::Acquire))
            .unwrap()
            .is_none()
    );
    assert_eq!(app.snapshot().unwrap().id(), old.id());
    assert_eq!(
        app.pinned(old.id().as_str()).unwrap().documents()[0].source(),
        Some("fn original() {}")
    );
    stop.store(false, Ordering::Release);
    let completed = engine(false)
        .refresh(Some(&old), &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(completed.measurements.parsed, 1);
    assert_eq!(completed.measurements.cache_hits, 0);
    assert_eq!(
        completed.snapshot.documents()[0].analyses()[0].facts[0].name,
        "changed"
    );
    assert_eq!(
        completed.snapshot.id(),
        engine(false)
            .refresh(None, &|| false)
            .unwrap()
            .unwrap()
            .snapshot
            .id()
    );
}

#[test]
fn adapter_returning_no_result_cannot_publish_a_snapshot_with_missing_files() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn original() {}").unwrap();
    let engine = Engine::new(
        Directory::with_languages(
            root.path(),
            Arc::new(
                Languages::new(vec![Arc::new(Adapter {
                    stop: Arc::new(AtomicBool::new(true)),
                    request: false,
                })])
                .unwrap(),
            ),
        )
        .unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    assert!(engine.refresh(None, &|| false).unwrap().is_none());
}
