//! Admitted review analysis uses existing adapters/cache without replacing source owners.

use std::{collections::BTreeMap, sync::Arc};
use vizzi::{
    cache::Cache,
    engine::Engine,
    review::{ReviewError, ReviewStore},
    review_source::{
        CaptureError, CaptureLimits, CaptureNotice, CapturePolicy, ReadEntry, ReviewInput,
    },
    source::Directory,
};
use vizzi_core::{
    comparison::{EntryKind, ReviewPath, Side},
    model::{Coverage, WorkspaceId},
    policy::Classification,
    projection::WindowQuery,
};

fn path(name: &str) -> ReviewPath {
    ReviewPath::parse(name.as_bytes().to_vec()).unwrap()
}
struct Input {
    workspace: WorkspaceId,
    files: BTreeMap<ReviewPath, Vec<u8>>,
}
impl Input {
    fn files(engine: &Engine, files: &[(&str, &str)]) -> Self {
        Self {
            workspace: engine.directory().workspace().clone(),
            files: files
                .iter()
                .map(|(name, text)| (path(name), text.as_bytes().to_vec()))
                .collect(),
        }
    }
}
impl ReviewInput for Input {
    fn workspace(&self) -> &WorkspaceId {
        &self.workspace
    }
    fn paths(
        &mut self,
        _: CaptureLimits,
        _: &dyn Fn() -> bool,
    ) -> Result<Vec<ReviewPath>, CaptureError> {
        Ok(self.files.keys().cloned().collect())
    }
    fn read(
        &mut self,
        path: &ReviewPath,
        limit: usize,
        _: &dyn Fn() -> bool,
    ) -> Result<ReadEntry, CaptureError> {
        let Some(bytes) = self.files.get(path) else {
            return Ok(ReadEntry::Absent);
        };
        if bytes.len() > limit {
            return Err(CaptureError::Limit("content bytes"));
        }
        Ok(ReadEntry::Present {
            kind: EntryKind::Regular { executable: false },
            bytes: bytes.clone(),
        })
    }
    fn notices(&self) -> Vec<CaptureNotice> {
        vec![]
    }
}

#[test]
fn mixed_review_facts_and_cache_reuse_keep_all_source_and_ownership() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        2,
    )
    .unwrap();
    let mut base = Input::files(
        &engine,
        &[
            ("a.rs", "fn before() {}"),
            ("tool.py", "def helper(): pass\n"),
            ("guide.md", "documentation\n"),
            ("legacy.c", "LPC source stays unassigned"),
        ],
    );
    let mut working = Input::files(
        &engine,
        &[
            ("a.rs", "fn after() {}"),
            ("tool.py", "def helper(): pass\n"),
            ("guide.md", "documentation\n"),
            ("legacy.c", "LPC source stays unassigned"),
        ],
    );
    let store = ReviewStore::open(1024).unwrap();
    let account = store.accounting();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut base,
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(&mut working, CaptureLimits::default(), &|| false)
        .unwrap();
    let before = account.stats();
    let measured = candidate.analyze(&engine, &|| false).unwrap();
    assert_eq!(measured.base.unwrap().parsed, 2);
    assert_eq!(measured.working.parsed, 1);
    assert_eq!(measured.working.cache_hits, 1);
    assert_eq!(account.stats(), before);
    let token = candidate.publish().unwrap();
    let lease = store.query(&token).unwrap();
    assert!(lease.analysis_plan().is_some());
    assert_eq!(
        lease
            .window(Side::Base, &path("a.rs"), &WindowQuery::default())
            .unwrap()
            .outline[0]
            .name,
        "before"
    );
    assert_eq!(
        lease
            .window(Side::Working, &path("a.rs"), &WindowQuery::default())
            .unwrap()
            .outline[0]
            .name,
        "after"
    );
    assert_eq!(
        lease.analyses(Side::Working, &path("tool.py")).unwrap()[0]
            .context
            .language,
        "python"
    );
    assert!(matches!(
        lease
            .classification(Side::Working, &path("guide.md"))
            .unwrap(),
        Classification::SourceOnly { .. }
    ));
    assert!(
        lease
            .analyses(Side::Working, &path("guide.md"))
            .unwrap()
            .is_empty()
    );
    assert!(
        lease
            .analyses(Side::Working, &path("legacy.c"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(lease.entries().count(), 4);
}

#[test]
fn analysed_refresh_keeps_baseline_facts_and_requires_analysis_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    let store = ReviewStore::open(1024).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &[("a.rs", "fn base() {}")]),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &[("a.rs", "fn one() {}")]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate.analyze(&engine, &|| false).unwrap();
    let token = candidate.publish().unwrap();
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(
            &mut Input::files(&engine, &[("a.rs", "fn two() {}")]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(matches!(refresh.publish(), Err(ReviewError::Stage)));
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(
            &mut Input::files(&engine, &[("a.rs", "fn two() {}")]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(refresh.analyze(&engine, &|| false).unwrap().base.is_none());
    let next = refresh.publish().unwrap();
    let lease = store.query(&next).unwrap();
    assert_eq!(
        lease
            .window(Side::Base, &path("a.rs"), &WindowQuery::default())
            .unwrap()
            .outline[0]
            .name,
        "base"
    );
    assert_eq!(
        lease
            .window(Side::Working, &path("a.rs"), &WindowQuery::default())
            .unwrap()
            .outline[0]
            .name,
        "two"
    );
}

#[test]
fn deferred_analysis_uses_the_shared_input_admission_and_keeps_source_inspectable() {
    use vizzi::admission::{Admission, Priority};
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let budget = Arc::new(Admission::new(64, 48, 16).unwrap());
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        2,
    )
    .unwrap()
    .with_admission(budget.clone());
    let permit = budget.acquire(48, Priority::Foreground).unwrap();
    let store = ReviewStore::open(1024).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let files = [("a.rs", "fn admission() { let x = 0; }")];
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    let measured = candidate.analyze(&engine, &|| false).unwrap();
    assert_eq!(measured.base.unwrap().deferred, 1);
    assert_eq!(measured.working.deferred, 1);
    assert_eq!(measured.working.parsed, 0);
    assert_eq!(measured.working.files, 1);
    assert_eq!(measured.working.analyzed, 1);
    assert_eq!(measured.working.source_only, 0);
    let token = candidate.publish().unwrap();
    let lease = store.query(&token).unwrap();
    assert_eq!(
        lease.analyses(Side::Working, &path("a.rs")).unwrap()[0].coverage,
        Coverage::Deferred
    );
    let window = lease
        .window(Side::Working, &path("a.rs"), &WindowQuery::default())
        .unwrap();
    assert_eq!(window.lines[0].text, files[0].1);
    assert!(window.outline.is_empty());
    drop(permit);
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(
        candidate
            .analyze(&engine, &|| false)
            .unwrap()
            .working
            .deferred,
        0
    );
    let token = candidate.publish().unwrap();
    assert_eq!(
        store
            .query(&token)
            .unwrap()
            .analyses(Side::Working, &path("a.rs"))
            .unwrap()[0]
            .coverage,
        Coverage::Syntax
    );
}

#[test]
fn c_family_assignment_stays_explicit_and_review_docs_survive_analysis_exclusion() {
    use vizzi_core::{
        analysis::Languages,
        policy::{PolicyConfig, SourceOnlyReason},
    };
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let directory = Directory::with_policy(
        root.path(),
        PolicyConfig {
            c_family_roots: vec!["native".into()],
            exclusions: vec!["legacy".into()],
            ..PolicyConfig::default()
        },
        Arc::new(Languages::builtin().unwrap()),
    )
    .unwrap();
    let engine = Engine::new(directory, Cache::new(cache.path().into()), 2).unwrap();
    let files = [
        ("native/a.c", "int c_function(void) { return 0; }"),
        ("native/b.cpp", "int cpp_function() { return 0; }"),
        ("native/h.h", "int declared();"),
        ("legacy/a.c", "LPC source"),
        ("doc.md", "text"),
    ];
    let store = ReviewStore::open(4096).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate.analyze(&engine, &|| false).unwrap();
    let token = candidate.publish().unwrap();
    let lease = store.query(&token).unwrap();
    assert_eq!(
        lease.analyses(Side::Base, &path("native/a.c")).unwrap()[0]
            .context
            .language,
        "c"
    );
    assert_eq!(
        lease.analyses(Side::Base, &path("native/b.cpp")).unwrap()[0]
            .context
            .language,
        "cpp"
    );
    assert!(matches!(
        lease
            .classification(Side::Base, &path("native/h.h"))
            .unwrap(),
        Classification::SourceOnly {
            reason: SourceOnlyReason::LanguageUnassigned,
            ..
        }
    ));
    for name in ["legacy/a.c", "doc.md"] {
        assert!(matches!(
            lease.classification(Side::Base, &path(name)).unwrap(),
            Classification::SourceOnly {
                reason: SourceOnlyReason::OutsideAnalysisScope,
                ..
            }
        ));
        assert!(lease.analyses(Side::Base, &path(name)).unwrap().is_empty());
        assert_eq!(
            lease
                .window(Side::Base, &path(name), &WindowQuery::default())
                .unwrap()
                .outline_total,
            0
        );
    }
}

#[test]
fn cancelled_analysis_does_not_publish_source_only_or_replace_existing_pair() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        1,
    )
    .unwrap();
    let store = ReviewStore::open(1024).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let files = [("a.rs", "fn original() {}")];
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    let old = candidate.publish().unwrap();
    assert!(store.query(&old).unwrap().analysis_plan().is_none());
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(matches!(
        candidate.analyze(&engine, &|| true),
        Err(ReviewError::Capture(CaptureError::Cancelled))
    ));
    assert!(matches!(candidate.publish(), Err(ReviewError::Stage)));
    assert_eq!(store.active().as_deref(), Some(old.as_str()));
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&old)).unwrap();
    refresh
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(matches!(
        refresh.analyze(&engine, &|| false),
        Err(ReviewError::AnalysisPolicy)
    ));
    assert!(matches!(refresh.publish(), Err(ReviewError::Stage)));
}

#[test]
fn host_review_entry_uses_the_host_engine_without_running_project_tooling() {
    use std::time::Duration;
    use vizzi::host::{Host, HostOptions};
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().into()),
        2,
    )
    .unwrap();
    let mut base = Input::files(&engine, &[("a.rs", "fn host_base() {}")]);
    let mut working = Input::files(&engine, &[("a.rs", "fn host_working() {}")]);
    let host = Host::start_with_options(
        engine,
        HostOptions {
            watch: false,
            reconcile_interval: Duration::from_secs(3600),
            idle_maintenance: false,
        },
    )
    .unwrap();
    let store = ReviewStore::open(1024).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut base,
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(&mut working, CaptureLimits::default(), &|| false)
        .unwrap();
    host.analyze_review(&mut candidate, &|| false).unwrap();
    let token = candidate.publish().unwrap();
    assert_eq!(
        store
            .query(&token)
            .unwrap()
            .window(Side::Working, &path("a.rs"), &WindowQuery::default())
            .unwrap()
            .outline[0]
            .name,
        "host_working"
    );
    assert_eq!(host.analysis_workers().peak, 1);
    assert_eq!(host.analysis_workers().active, 0);
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(&mut working, CaptureLimits::default(), &|| false)
        .unwrap();
    assert!(matches!(
        host.analyze_review(&mut refresh, &|| true),
        Err(ReviewError::Capture(CaptureError::Cancelled))
    ));
    assert!(matches!(refresh.publish(), Err(ReviewError::Stage)));
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
}

#[derive(Default)]
struct Control {
    fail: std::sync::atomic::AtomicBool,
    panic: std::sync::atomic::AtomicBool,
    block: std::sync::atomic::AtomicBool,
    active: std::sync::atomic::AtomicUsize,
    peak: std::sync::atomic::AtomicUsize,
    started: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<()>>>,
}
struct ControlledAdapter {
    control: Arc<Control>,
    version: &'static str,
}
impl vizzi_core::analysis::LanguageAdapter for ControlledAdapter {
    fn context(&self) -> vizzi_core::model::Context {
        vizzi_core::model::Context {
            language: "rust".into(),
            description: self.version.into(),
            id: vizzi_core::model::ContextId::derive(&[self.version.as_bytes()]),
        }
    }
    fn create(
        &self,
    ) -> Result<Box<dyn vizzi_core::analysis::Analyzer>, vizzi_core::model::ModelError> {
        Ok(Box::new(ControlledParser {
            control: self.control.clone(),
            context: self.context(),
            parser: vizzi_core::rust::RustAnalyzer::new()?,
        }))
    }
}
struct ControlledParser {
    control: Arc<Control>,
    context: vizzi_core::model::Context,
    parser: vizzi_core::rust::RustAnalyzer,
}
struct Active(Arc<Control>);
impl Drop for Active {
    fn drop(&mut self) {
        self.0
            .active
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
struct Release(Arc<Control>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0
            .block
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
impl vizzi_core::analysis::Analyzer for ControlledParser {
    fn analyze(
        &mut self,
        bytes: &[u8],
    ) -> Result<vizzi_core::model::Analysis, vizzi_core::model::ModelError> {
        self.analyze_cancellable(bytes, &|| false)?
            .ok_or_else(|| vizzi_core::model::ModelError("unexpected cancellation".into()))
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<vizzi_core::model::Analysis>, vizzi_core::model::ModelError> {
        use std::sync::atomic::Ordering::SeqCst;
        let active = self.control.active.fetch_add(1, SeqCst) + 1;
        self.control.peak.fetch_max(active, SeqCst);
        let _active = Active(self.control.clone());
        let is_review = bytes
            .windows(b"review".len())
            .any(|value| value == b"review");
        if is_review {
            if let Some(started) = self.control.started.lock().unwrap().take() {
                started.send(()).unwrap();
            }
            while self.control.block.load(SeqCst) {
                if cancelled() {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }
        if self.control.panic.load(SeqCst) {
            panic!("fixture parser panic");
        }
        if self.control.fail.load(SeqCst) {
            return Err(vizzi_core::model::ModelError(
                "fixture analysis failure".into(),
            ));
        }
        if cancelled() {
            return Ok(None);
        }
        let mut result = self.parser.analyze(bytes)?;
        result.context = self.context.clone();
        Ok(Some(result))
    }
}
fn controlled_engine(
    root: &std::path::Path,
    cache: &std::path::Path,
    control: Arc<Control>,
    version: &'static str,
    workers: usize,
) -> Engine {
    let languages = vizzi_core::analysis::Languages::new(vec![Arc::new(ControlledAdapter {
        control,
        version,
    })])
    .unwrap();
    Engine::new(
        Directory::with_languages(root, Arc::new(languages)).unwrap(),
        Cache::new(cache.into()),
        workers,
    )
    .unwrap()
}

#[test]
fn failed_and_panicking_analysis_cannot_publish_and_refresh_cannot_change_contexts() {
    use std::sync::atomic::Ordering::SeqCst;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let control = Arc::new(Control::default());
    let engine = controlled_engine(root.path(), cache.path(), control.clone(), "v1", 2);
    let store = ReviewStore::open(4096).unwrap();
    let account = store.accounting();
    let files = [("a.rs", "fn failure_case() {}")];
    for panics in [false, true] {
        let ticket = store.request().unwrap();
        let mut candidate = store.begin(&ticket, None).unwrap();
        candidate
            .capture_base(
                &mut Input::files(&engine, &files),
                &CapturePolicy::scope(None, vec![]),
                CaptureLimits::default(),
                &|| false,
            )
            .unwrap();
        candidate
            .capture_working(
                &mut Input::files(&engine, &files),
                CaptureLimits::default(),
                &|| false,
            )
            .unwrap();
        control.fail.store(!panics, SeqCst);
        control.panic.store(panics, SeqCst);
        assert!(matches!(
            candidate.analyze(&engine, &|| false),
            Err(ReviewError::Analysis(_))
        ));
        assert!(matches!(candidate.publish(), Err(ReviewError::Stage)));
        assert!(store.active().is_none());
        assert_eq!(account.stats().charged_bytes, 0);
        assert_eq!(engine.analysis_workers().active, 0);
    }
    control.fail.store(false, SeqCst);
    control.panic.store(false, SeqCst);
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate.analyze(&engine, &|| false).unwrap();
    let token = candidate.publish().unwrap();
    let changed = controlled_engine(root.path(), cache.path(), control, "v2", 2);
    let ticket = store.request().unwrap();
    let mut refresh = store.begin(&ticket, Some(&token)).unwrap();
    refresh
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    assert!(matches!(
        refresh.analyze(&changed, &|| false),
        Err(ReviewError::AnalysisPolicy)
    ));
    assert!(matches!(refresh.publish(), Err(ReviewError::Stage)));
    assert_eq!(store.active().as_deref(), Some(token.as_str()));
}

#[test]
fn review_and_foreground_parsers_share_the_same_aggregate_capacity() {
    use std::sync::atomic::Ordering::SeqCst;
    use std::time::Duration;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("live.rs"), "fn live() {}").unwrap();
    let control = Arc::new(Control::default());
    control.block.store(true, SeqCst);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    *control.started.lock().unwrap() = Some(tx);
    let engine = Arc::new(controlled_engine(
        root.path(),
        cache.path(),
        control.clone(),
        "concurrent",
        2,
    ));
    let store = ReviewStore::open(1024).unwrap();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    candidate
        .capture_base(
            &mut Input::files(&engine, &[("a.rs", "fn review_base() {}")]),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &[("a.rs", "fn review_working() {}")]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    std::thread::scope(|scope| {
        let analysis_engine = engine.clone();
        let task = scope.spawn(move || candidate.analyze(&analysis_engine, &|| false));
        let release = Release(control.clone());
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(engine.analysis_workers().active, 1);
        assert!(engine.analysis_workers().background);
        let live = engine.refresh(None, &|| false).unwrap().unwrap();
        assert_eq!(live.measurements.parsed, 1);
        assert_eq!(engine.analysis_workers().active, 1);
        assert_eq!(engine.analysis_workers().peak, 2);
        drop(release);
        task.join().unwrap().unwrap();
    });
    assert_eq!(control.peak.load(SeqCst), 2);
    assert_eq!(engine.analysis_workers().active, 0);
}

#[test]
fn superseded_review_is_cancelled_inside_parsing_and_cannot_replace_the_pair() {
    use std::sync::atomic::Ordering::SeqCst;
    use std::time::Duration;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let control = Arc::new(Control::default());
    let engine = Arc::new(controlled_engine(
        root.path(),
        cache.path(),
        control.clone(),
        "superseded",
        1,
    ));
    let store = ReviewStore::open(1024).unwrap();
    let account = store.accounting();
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let files = [("a.rs", "fn original() {}")];
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    let old = candidate.publish().unwrap();
    let before = account.stats().retained_bytes;
    let ticket = store.request().unwrap();
    let mut candidate = store.begin(&ticket, None).unwrap();
    let files = [("a.rs", "fn review_changed() {}")];
    candidate
        .capture_base(
            &mut Input::files(&engine, &files),
            &CapturePolicy::scope(None, vec![]),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    candidate
        .capture_working(
            &mut Input::files(&engine, &files),
            CaptureLimits::default(),
            &|| false,
        )
        .unwrap();
    control.block.store(true, SeqCst);
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    *control.started.lock().unwrap() = Some(tx);
    std::thread::scope(|scope| {
        let analysis_engine = engine.clone();
        let task = scope.spawn(move || {
            let result = candidate.analyze(&analysis_engine, &|| false);
            (candidate, result)
        });
        let _release = Release(control.clone());
        rx.recv_timeout(Duration::from_secs(2)).unwrap();
        store.request().unwrap();
        let (candidate, result) = task.join().unwrap();
        assert!(matches!(result, Err(ReviewError::Stale)));
        assert!(matches!(candidate.publish(), Err(ReviewError::Stale)));
    });
    assert_eq!(store.active().as_deref(), Some(old.as_str()));
    assert_eq!(account.stats().retained_bytes, before);
    assert_eq!(account.stats().pair_slots, 1);
    assert_eq!(engine.analysis_workers().active, 0);
}

#[test]
fn simultaneous_foreground_parser_panics_are_errors_and_release_shared_capacity() {
    use std::sync::atomic::Ordering::SeqCst;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "fn one() {}").unwrap();
    std::fs::write(root.path().join("b.rs"), "fn two() {}").unwrap();
    let control = Arc::new(Control::default());
    control.panic.store(true, SeqCst);
    let engine = controlled_engine(root.path(), cache.path(), control, "two-panics", 2);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        engine.refresh(None, &|| false)
    }));
    assert!(matches!(
        result,
        Ok(Err(vizzi::engine::EngineError::Configuration(_)))
    ));
    assert_eq!(engine.analysis_workers().active, 0);
}
