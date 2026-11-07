//! Thin investigation and verified-editor HTTP adapter.
use crate::companion::{Companion, CompanionError, EditInvestigation, PinEvidence};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;
static WORK: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
static EDITORS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
#[derive(Debug)]
struct Error(CompanionError);
impl From<CompanionError> for Error {
    fn from(e: CompanionError) -> Self {
        Self(e)
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self.0 {
            CompanionError::Invalid(_) => StatusCode::BAD_REQUEST,
            CompanionError::Conflict(_) => StatusCode::CONFLICT,
            CompanionError::Unavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            CompanionError::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
            CompanionError::Editor(_) => StatusCode::BAD_GATEWAY,
        };
        let message = crate::review_application::clip(&self.0.to_string(), 512).0;
        (status, Json(serde_json::json!({"message":message}))).into_response()
    }
}
pub(super) fn router(
    app: Arc<Companion>,
    search: Arc<crate::question_search::QuestionSearch>,
) -> Router {
    Router::new()
        .route("/api/investigation", get(view).post(edit))
        .route("/api/discovery", get(discover))
        .route(
            "/api/search",
            get(|| async {
                Json(serde_json::json!({"trial":true,"question_bytes":2048,"result_limit":8}))
            })
            .post(
                move |Json(request): Json<crate::question_search::SearchRequest>| {
                    let search = search.clone();
                    async move { work(move || search.search(&request)).await }
                },
            ),
        )
        .route("/api/search/evidence", post(pin_current))
        .route("/api/investigation/lookup", get(lookup))
        .route("/api/investigation/evidence", post(pin))
        .route("/api/investigation/evidence/{id}/open", post(open))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                response.headers_mut().insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    header::HeaderValue::from_static("nosniff"),
                );
                response
            },
        ))
        .with_state(app)
}
async fn work<T: Send + 'static>(
    job: impl FnOnce() -> Result<T, CompanionError> + Send + 'static,
) -> Result<Json<T>, Error> {
    let permit = WORK.try_acquire().map_err(|_| {
        Error(CompanionError::Unavailable(
            "Investigation operation is busy; retry".into(),
        ))
    })?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        job().map(Json).map_err(Error)
    })
    .await
    .map_err(|e| Error(CompanionError::Unavailable(e.to_string())))?
}
async fn view(State(app): State<Arc<Companion>>) -> Json<crate::companion::CompanionView> {
    Json(app.view())
}
async fn discover(
    State(app): State<Arc<Companion>>,
    Query(query): Query<vizzi_core::discovery::DiscoveryQuery>,
) -> Result<Json<vizzi_core::discovery::Discovery>, Error> {
    work(move || app.discover(&query)).await
}
#[derive(Deserialize)]
struct Lookup {
    path: String,
}
async fn lookup(
    State(app): State<Arc<Companion>>,
    Query(query): Query<Lookup>,
) -> Result<Json<crate::companion::SourceMatch>, Error> {
    work(move || app.lookup(&query.path)).await
}
async fn edit(
    State(app): State<Arc<Companion>>,
    Json(edit): Json<EditInvestigation>,
) -> Result<Json<crate::companion::CompanionView>, Error> {
    work(move || {
        app.edit(edit)?;
        Ok(app.view())
    })
    .await
}
async fn pin(
    State(app): State<Arc<Companion>>,
    Json(request): Json<PinEvidence>,
) -> Result<Json<serde_json::Value>, Error> {
    work(move || {
        let id = app.pin(request)?;
        Ok(serde_json::json!({"id":id,"view":app.view()}))
    })
    .await
}
async fn pin_current(
    State(app): State<Arc<Companion>>,
    Json(request): Json<PinEvidence>,
) -> Result<Json<serde_json::Value>, Error> {
    work(move || {
        let id = app.pin_current(request)?;
        Ok(serde_json::json!({"id":id,"view":app.view()}))
    })
    .await
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn open(
    State(app): State<Arc<Companion>>,
    Path(id): Path<u32>,
    Json(_): Json<Empty>,
) -> Result<Json<serde_json::Value>, Error> {
    let permit = EDITORS.try_acquire().map_err(|_| {
        Error(CompanionError::Unavailable(
            "An editor handoff is already running".into(),
        ))
    })?;
    work(move || {
        let _permit = permit;
        app.open(id)?;
        Ok(serde_json::json!({"requested":true}))
    })
    .await
}
