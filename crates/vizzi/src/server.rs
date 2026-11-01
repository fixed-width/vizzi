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

type ApiError = (StatusCode, String);

/// Builds the local browser host, also usable through in-process requests.
pub fn router(application: Arc<Application>) -> Router {
    Router::new()
        .route("/api/status", get(status)).route("/api/view", get(view)).route("/api/source", get(source))
        .route("/api/events", get(events))
        .route("/", get(|| async { asset("text/html; charset=utf-8", include_str!("../web/index.html")) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/app.js")) }))
        .route("/controller.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/controller.js")) }))
        .route("/updates.js", get(|| async { asset("text/javascript; charset=utf-8", include_str!("../web/updates.js")) }))
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
    Ok(Json(snapshot.view(&query)))
}
#[derive(Deserialize)]
struct SourceQuery {
    snapshot: String,
    file: String,
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
    Ok(Json(
        serde_json::json!({ "schema": 1, "snapshot": snapshot.id(), "file": file.id(), "path": file.path(), "text": file.source() }),
    ))
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
    use tower::ServiceExt;
    let request = axum::http::Request::builder()
        .uri(uri)
        .body(Body::empty())?;
    if request.uri().path() == "/api/events" {
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
        assert!(page.body.contains("canvas"));
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
