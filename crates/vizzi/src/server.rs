//! HTTP and diagnostic transports over the same read-only application handlers.

use crate::application::Application;
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use vizzi_core::query::{View, ViewQuery};
use vizzi_core::{
    model::SnapshotId,
    projection::{Window, WindowQuery},
};

type ApiError = (StatusCode, String);
mod companion;
mod review;

/// Serves deferred HTTP exploration and review through one native host.
pub fn router_with_host(
    host: Arc<crate::host::Host>,
    reviews: Arc<crate::review_application::ReviewApplication>,
) -> Router {
    let activation = Router::new()
        .route("/api/workspace/start", axum::routing::post(start_workspace))
        .layer(axum::extract::DefaultBodyLimit::max(1024))
        .with_state(host.clone());
    router_with_reviews(host.application(), reviews).merge(activation)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartWorkspace {}
async fn start_workspace(
    State(host): State<Arc<crate::host::Host>>,
    Json(_): Json<StartWorkspace>,
) -> impl IntoResponse {
    host.activate();
    (
        StatusCode::ACCEPTED,
        [
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Json(host.application().status()),
    )
}

/// Adds investigation and editor operations to the host-aware local transport.
pub fn router_with_companion(
    host: Arc<crate::host::Host>,
    reviews: Arc<crate::review_application::ReviewApplication>,
    investigation: Arc<crate::companion::Companion>,
) -> Router {
    router_with_host(host, reviews).merge(companion::router(investigation))
}

/// Adds comparison operations to the live workspace router without sharing status schemas.
pub fn router_with_reviews(
    application: Arc<Application>,
    reviews: Arc<crate::review_application::ReviewApplication>,
) -> Router {
    router(application).merge(review::router(reviews))
}

/// Builds the local browser host, also usable through in-process requests.
pub fn router(application: Arc<Application>) -> Router {
    Router::new()
        .route("/api/status", get(status)).route("/api/view", get(view)).route("/api/source", get(source))
        .route("/api/events", get(events)).route("/api/window",get(window))
        .route("/", get(|| async { asset("text/html; charset=utf-8", include_str!("../web/companion.html")) }))
        .route("/explore", get(|| async { asset("text/html; charset=utf-8", include_str!("../web/index.html")) }))
        .route("/investigate", get(|| async { asset("text/html; charset=utf-8", include_str!("../web/companion.html")) }))
        .route("/companion.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/companion.js")) }))
        .route("/companion.css", get(|| async { asset("text/css; charset=utf-8", include_str!("../web/companion.css")) }))
        .route("/review", get(|| async { asset("text/html; charset=utf-8", include_str!("../web/review.html")) }))
        .route("/review-app.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/review-app.js")) }))
        .route("/review-controller.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/review-controller.js")) }))
        .route("/review-windows.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/review-windows.js")) }))
        .route("/review.css", get(|| async { asset("text/css; charset=utf-8", include_str!("../web/review.css")) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/app.js")) }))
        .route("/startup.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/startup.js")) }))
        .route("/controller.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/controller.js")) }))
        .route("/navigation.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/navigation.js")) }))
        .route("/updates.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/updates.js")) }))
        .route("/source-description.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/source-description.js")) }))
        .route("/outline.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/outline.js")) }))
        .route("/windows.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/windows.js")) }))
        .route("/inspector.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/inspector.js")) }))
        .route("/presentation.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/presentation.js")) }))
        .route("/camera.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/camera.js")) }))
        .route("/splitter.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/splitter.js")) }))
        .route("/map-scene.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/map-scene.js")) }))
        .route("/directory-controls.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/directory-controls.js")) }))
        .route("/map-headings.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/map-headings.js")) }))
        .route("/view-status.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/view-status.js")) }))
        .route("/messages.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/messages.js")) }))
        .route("/appearance.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/appearance.js")) }))
        .route("/theme-init.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/theme-init.js")) }))
        .route("/theme.css", get(|| async { asset("text/css; charset=utf-8", include_str!("../web/theme.css")) }))
        .route("/app.css", get(|| async { asset("text/css; charset=utf-8", include_str!("../web/app.css")) }))
        .layer(axum::middleware::map_response(|mut response: Response| async move {
            response.headers_mut().insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
            response.headers_mut().insert(header::CONTENT_SECURITY_POLICY, header::HeaderValue::from_static("default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; object-src 'none'; frame-ancestors 'none'"));
            response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, header::HeaderValue::from_static("nosniff"));
            response
        }))
        .with_state(application)
}
async fn status(State(app): State<Arc<Application>>) -> Json<crate::application::Status> {
    Json(app.status())
}
async fn events(State(app): State<Arc<Application>>) -> impl IntoResponse {
    let stream = futures_util::stream::unfold(app.subscribe(), |mut subscription| async move {
        let change = subscription.next().await?;
        let event = Event::default()
            .event("workspace")
            .id(change.id())
            .json_data(&*change);
        Some((event, subscription))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
#[derive(Deserialize)]
struct VersionQuery {
    snapshot: Option<String>,
}
static SOURCE_READS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
fn source_error(error: vizzi_core::model::SourceReadError) -> ApiError {
    use vizzi_core::model::SourceReadError;
    let status = match error {
        SourceReadError::Capacity => StatusCode::PAYLOAD_TOO_LARGE,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    };
    (status, error.to_string())
}
async fn source_work<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, vizzi_core::model::SourceReadError> + Send + 'static,
) -> Result<T, ApiError> {
    let permit = SOURCE_READS.try_acquire().map_err(|_| {
        (
            StatusCode::TOO_MANY_REQUESTS,
            "Source read capacity is busy; retry inspection".into(),
        )
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        work()
    })
    .await
    .map_err(|_| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "Source read worker failed".into(),
        )
    })?
    .map_err(source_error)
}
async fn view(
    State(app): State<Arc<Application>>,
    Query(version): Query<VersionQuery>,
    Query(query): Query<ViewQuery>,
) -> Result<Json<View>, ApiError> {
    let snapshot = if let Some(id) = version.snapshot {
        app.pinned(&id).ok_or((
            StatusCode::CONFLICT,
            "Snapshot expired or unknown; resynchronise updates".into(),
        ))?
    } else {
        app.snapshot().ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            "Initial capture is not ready".into(),
        ))?
    };
    Ok(Json(source_work(move || snapshot.view(&query)).await?))
}
#[derive(Deserialize)]
struct SourceQuery {
    snapshot: String,
    file: String,
}
#[derive(Deserialize)]
struct WindowPin {
    snapshot: String,
    file: String,
    version: String,
    previous_snapshot: Option<String>,
}
#[derive(serde::Serialize)]
struct PinnedWindow {
    snapshot: SnapshotId,
    #[serde(flatten)]
    window: Window,
}
async fn window(
    State(app): State<Arc<Application>>,
    Query(pin): Query<WindowPin>,
    Query(query): Query<WindowQuery>,
) -> Result<Json<PinnedWindow>, ApiError> {
    let snapshot = app.pinned(&pin.snapshot).ok_or((
        StatusCode::CONFLICT,
        "Snapshot expired or unknown; reload inspection".into(),
    ))?;
    let file = snapshot.file(&pin.file).ok_or((
        StatusCode::NOT_FOUND,
        "File is absent from this snapshot".into(),
    ))?;
    if file.version().as_str() != pin.version {
        return Err((
            StatusCode::CONFLICT,
            "Document version differs; reload inspection".into(),
        ));
    }
    let previous = pin
        .previous_snapshot
        .as_deref()
        .and_then(|id| app.pinned(id));
    let previous_file = previous.as_ref().and_then(|old| old.file(&pin.file));
    let file = file.clone();
    let previous_file = previous_file.cloned();
    let snapshot_id = snapshot.id().clone();
    Ok(Json(
        source_work(move || {
            Ok(PinnedWindow {
                snapshot: snapshot_id,
                window: file.window(&query, previous_file.as_deref())?,
            })
        })
        .await?,
    ))
}
async fn source(
    State(app): State<Arc<Application>>,
    Query(query): Query<SourceQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let snapshot = app.pinned(&query.snapshot).ok_or((
        StatusCode::CONFLICT,
        "Snapshot expired or unknown; reload the view".into(),
    ))?;
    let file = snapshot.file(&query.file).ok_or((
        StatusCode::NOT_FOUND,
        "File is absent from this snapshot".into(),
    ))?;
    let file = file.clone();
    Ok(Json(source_work(move|| {
        let text=file.captured().read_text(vizzi_core::model::BULK_SOURCE_LIMIT)?.map(|text|text.into_owned());
        Ok(serde_json::json!({ "schema": 2, "snapshot": snapshot.id(), "policy": snapshot.policy(), "file": file.id(), "version": file.version(), "path": file.path(), "text": text, "encoding": file.encoding(), "classification": file.classification() }))
    }).await?))
}
fn asset(content_type: &'static str, text: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], text).into_response()
}

/// Diagnostic response containing the exact HTTP handler result.
#[derive(serde::Serialize, Deserialize)]
pub struct DiagnosticResponse {
    /// Caller-supplied correlation identity.
    pub id: serde_json::Value,
    /// HTTP status code.
    pub status: u16,
    /// Response content type, when set.
    pub content_type: String,
    /// UTF-8 response body.
    pub body: String,
}
/// Dispatches one diagnostic request through the normal HTTP router.
pub async fn diagnostic(
    router: Router,
    id: serde_json::Value,
    uri: &str,
) -> Result<DiagnosticResponse, Box<dyn std::error::Error>> {
    diagnostic_request(router, id, "GET", uri, None).await
}

/// Dispatches a finite diagnostic mutation/read through the same local HTTP handlers.
pub async fn diagnostic_request(
    router: Router,
    id: serde_json::Value,
    method: &str,
    uri: &str,
    body: Option<serde_json::Value>,
) -> Result<DiagnosticResponse, Box<dyn std::error::Error>> {
    use tower::ServiceExt;
    let body = body
        .map(|body| serde_json::to_vec(&body))
        .transpose()?
        .unwrap_or_default();
    let request = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))?;
    if matches!(
        request.uri().path(),
        "/api/events" | "/api/comparisons/events"
    ) {
        return Ok(DiagnosticResponse { id, status: 400, content_type: "text/plain".into(), body: "Event streaming requires HTTP or a native subscription; diagnostics return finite bodies".into() });
    }
    let response = router.oneshot(request).await?;
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let body = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 128 * 1024 * 1024)
            .await?
            .to_vec(),
    )?;
    Ok(DiagnosticResponse {
        id,
        status,
        content_type,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cache::Cache, engine::Engine, source::Directory};
    #[tokio::test]
    async fn handlers_pin_source_across_edits_and_diagnostics_share_results() {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.rs"), "fn original() {}").unwrap();
        let engine = Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap();
        let application = Arc::new(Application::new("fixture".into()).unwrap());
        let ticket = application.request();
        application.publish(ticket, engine.refresh(None, &|| false).unwrap().unwrap());
        let router = router(application.clone());
        let response = diagnostic(router.clone(), serde_json::json!(1), "/api/view")
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert!(response.content_type.contains("application/json"));
        let view: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(view["source"]["text"], "fn original() {}");
        let pinned_uri = format!(
            "/api/source?snapshot={}&file={}",
            view["snapshot"].as_str().unwrap(),
            view["source"]["id"].as_str().unwrap()
        );
        std::fs::write(root.path().join("a.rs"), "fn edited() {}").unwrap();
        let ticket = application.request();
        application.publish(
            ticket,
            engine
                .refresh(application.snapshot().as_deref(), &|| false)
                .unwrap()
                .unwrap(),
        );
        let pinned = diagnostic(router.clone(), serde_json::json!(2), &pinned_uri)
            .await
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&pinned.body).unwrap()["text"],
            "fn original() {}"
        );
        let pinned_view = diagnostic(
            router.clone(),
            serde_json::json!(6),
            &pinned_uri.replacen("/api/source", "/api/view", 1),
        )
        .await
        .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&pinned_view.body).unwrap()["source"]["text"],
            "fn original() {}"
        );
        assert_eq!(
            diagnostic(
                router.clone(),
                serde_json::Value::Null,
                "/api/view?snapshot=expired"
            )
            .await
            .unwrap()
            .status,
            409
        );
        let expired = diagnostic(
            router.clone(),
            serde_json::json!(3),
            "/api/source?snapshot=unknown&file=unknown",
        )
        .await
        .unwrap();
        assert_eq!(expired.status, 409);
        let malformed = diagnostic(router.clone(), serde_json::json!(4), "/api/view?limit=bad")
            .await
            .unwrap();
        assert_eq!(malformed.status, 400);
        let page = diagnostic(router.clone(), serde_json::json!(5), "/")
            .await
            .unwrap();
        assert_eq!(page.status, 200);
        assert!(page.body.contains("Investigation"));
        assert_eq!(
            diagnostic(router, serde_json::Value::Null, "/absent")
                .await
                .unwrap()
                .status,
            404
        );
    }

    #[tokio::test]
    async fn event_stream_delivers_latest_state_and_finishes_on_shutdown() {
        use futures_util::FutureExt;
        use http_body_util::BodyExt;
        use tower::ServiceExt;
        let app = Arc::new(Application::new("events".into()).unwrap());
        let response = router(app.clone())
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/events")
                    .header("Last-Event-ID", "obsolete:900")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
        let mut body = response.into_body();
        let first = body
            .frame()
            .now_or_never()
            .expect("stream frame should be ready")
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        let first = std::str::from_utf8(&first).unwrap();
        assert!(first.contains("event: workspace"));
        let notification: serde_json::Value = serde_json::from_str(
            first
                .lines()
                .find_map(|line| line.strip_prefix("data: "))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(notification["revision"], "0");
        assert_eq!(notification["status"]["name"], "events");
        assert!(first.contains(&format!(
            "id: {}:0",
            notification["stream"].as_str().unwrap()
        )));
        app.request();
        app.observation("ready".into());
        let next = body
            .frame()
            .now_or_never()
            .expect("stream frame should be ready")
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        let next = std::str::from_utf8(&next).unwrap();
        let next: serde_json::Value = serde_json::from_str(
            next.lines()
                .find_map(|line| line.strip_prefix("data: "))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(next["revision"], "2");
        assert_eq!(next["status"]["pending"], true);
        assert_eq!(next["status"]["observation"], "ready");
        app.close_updates();
        assert!(
            body.frame()
                .now_or_never()
                .expect("stream frame should be ready")
                .is_none()
        );
    }

    #[tokio::test]
    async fn diagnostics_reject_an_unbounded_stream() {
        let app = Arc::new(Application::new("events".into()).unwrap());
        let response = diagnostic(router(app), serde_json::json!(9), "/api/events?ignored=1")
            .await
            .unwrap();
        assert_eq!(response.id, 9);
        assert_eq!(response.status, 400);
        assert!(response.body.contains("stream"));
    }

    #[tokio::test]
    async fn diagnostic_bodies_can_exceed_one_megabyte() {
        let data = "x".repeat(2 * 1024 * 1024);
        let route = Router::new().route(
            "/large",
            get(move || {
                let data = data.clone();
                async move { data }
            }),
        );
        let response = diagnostic(route, serde_json::json!(1), "/large")
            .await
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body.len(), 2 * 1024 * 1024);
    }
}
