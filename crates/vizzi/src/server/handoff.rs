//! Finite adapters for the shared captured-source handoff module.
use crate::{
    companion::CompanionError,
    handoff::{Cancel, Handoffs, Request},
    host::Host,
    structural::StructureHost,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

pub(super) fn router(host: Arc<Host>, structure: Arc<StructureHost>) -> Router {
    Router::new()
        .route("/api/editor/handoff/window", post(window))
        .route("/api/editor/handoff", post(step))
        .route("/api/editor/handoff/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(16 * 1024 * 1024))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-store"),
                );
                response
            },
        ))
        .with_state(Arc::new(Handoffs::new(host, structure)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Window {
    root: PathBuf,
}
async fn window(State(handoffs): State<Arc<Handoffs>>, Json(request): Json<Window>) -> Response {
    match handoffs.attach_window(&request.root) {
        Ok(scope) => Json(serde_json::json!({"schema":1,"scope":scope})).into_response(),
        Err(error) => {
            let status = if matches!(error, CompanionError::Conflict(_)) {
                StatusCode::CONFLICT
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            (
                status,
                Json(serde_json::json!({"message":error.to_string()})),
            )
                .into_response()
        }
    }
}
async fn step(
    State(handoffs): State<Arc<Handoffs>>,
    Json(request): Json<Request>,
) -> Json<crate::handoff::Reply> {
    Json(handoffs.handoff(request).await)
}
async fn cancel(
    State(handoffs): State<Arc<Handoffs>>,
    Json(request): Json<Cancel>,
) -> Json<crate::handoff::Reply> {
    Json(handoffs.cancel(request))
}
