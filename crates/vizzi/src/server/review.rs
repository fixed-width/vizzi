//! Thin HTTP/stdio adapter for native comparison operations.

use crate::review_application::{
    CreateComparison, ReviewApiError, ReviewApplication, ReviewDiffQuery, ReviewPageQuery,
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{StatusCode, header},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;
use vizzi_core::{comparison::Side, projection::WindowQuery};

pub(super) fn router(app: Arc<ReviewApplication>) -> Router {
    Router::new()
        .route("/api/comparisons", axum::routing::post(create))
        .route("/api/comparisons/status", get(status))
        .route("/api/comparisons/events", get(events))
        .route("/api/comparisons/{token}", get(metadata).delete(close))
        .route(
            "/api/comparisons/{token}/refresh",
            axum::routing::post(refresh),
        )
        .route("/api/comparisons/{token}/files", get(files))
        .route("/api/comparisons/{token}/window", get(window))
        .route("/api/comparisons/{token}/diff", get(diff))
        .layer(DefaultBodyLimit::max(8192))
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn create(
    State(app): State<Arc<ReviewApplication>>,
    Json(request): Json<CreateComparison>,
) -> Result<impl IntoResponse, HttpError> {
    Ok((StatusCode::ACCEPTED, Json(app.create(request)?)))
}
async fn refresh(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
    Json(_): Json<Empty>,
) -> Result<impl IntoResponse, HttpError> {
    Ok((StatusCode::ACCEPTED, Json(app.refresh(&token)?)))
}
async fn close(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
    Json(_): Json<Empty>,
) -> Result<StatusCode, HttpError> {
    let _ = work(false, move || {
        app.close(&token)?;
        Ok(())
    })
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn status(
    State(app): State<Arc<ReviewApplication>>,
) -> Json<crate::review_application::ReviewStatus> {
    Json(app.status())
}
async fn events(State(app): State<Arc<ReviewApplication>>) -> impl IntoResponse {
    let stream = futures_util::stream::unfold(app.subscribe(), |mut receiver| async move {
        let change = receiver.next().await?;
        let event = Event::default()
            .event("comparison")
            .id(format!("{}:{}", change.stream, change.revision))
            .json_data(&*change);
        Some((event, receiver))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
async fn metadata(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
) -> Result<Json<crate::review_application::ComparisonMetadata>, HttpError> {
    work(false, move || Ok(app.query(&token)?.metadata().clone())).await
}

static QUERIES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
static DIFFS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
async fn work<T: Send + 'static>(
    diff: bool,
    operation: impl FnOnce() -> Result<T, ReviewApiError> + Send + 'static,
) -> Result<Json<T>, HttpError> {
    let diff_permit = if diff {
        Some(
            DIFFS
                .try_acquire()
                .map_err(|_| ReviewApiError::QueryCapacity)?,
        )
    } else {
        None
    };
    let permit = QUERIES
        .try_acquire()
        .map_err(|_| ReviewApiError::QueryCapacity)?;
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let _diff = diff_permit;
        operation()
    })
    .await
    .map_err(|_| ReviewApiError::Internal("query worker failed".into()))??;
    Ok(Json(result))
}
async fn files(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
    Query(page): Query<ReviewPageQuery>,
) -> Result<Json<crate::review_application::ReviewFiles>, HttpError> {
    work(false, move || app.query(&token)?.files(&page)).await
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireSide {
    Base,
    Working,
}
impl From<WireSide> for Side {
    fn from(side: WireSide) -> Self {
        match side {
            WireSide::Base => Self::Base,
            WireSide::Working => Self::Working,
        }
    }
}
#[derive(Deserialize)]
struct WindowPin {
    file: String,
    side: WireSide,
    version: String,
}
async fn window(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
    Query(pin): Query<WindowPin>,
    Query(page): Query<WindowQuery>,
) -> Result<Json<crate::review_application::ReviewWindow>, HttpError> {
    work(false, move || {
        app.query(&token)?
            .window(&pin.file, pin.side.into(), &pin.version, &page)
    })
    .await
}
#[derive(Deserialize)]
struct DiffPin {
    file: String,
    base_version: Option<String>,
    working_version: Option<String>,
}
async fn diff(
    State(app): State<Arc<ReviewApplication>>,
    Path(token): Path<String>,
    Query(pin): Query<DiffPin>,
    Query(page): Query<ReviewDiffQuery>,
) -> Result<Json<crate::review_application::ReviewDiff>, HttpError> {
    work(true, move || {
        app.query(&token)?.diff(
            &pin.file,
            pin.base_version.as_deref(),
            pin.working_version.as_deref(),
            &page,
        )
    })
    .await
}

#[derive(Debug)]
struct HttpError(ReviewApiError);
impl From<ReviewApiError> for HttpError {
    fn from(error: ReviewApiError) -> Self {
        Self(error)
    }
}
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        use crate::review::ReviewError;
        use vizzi_core::{model::SourceReadError, text_diff::DiffError};
        let status = match &self.0 {
            ReviewApiError::Invalid(_) => StatusCode::BAD_REQUEST,
            ReviewApiError::NotFound | ReviewApiError::Review(ReviewError::SourceUnavailable) => {
                StatusCode::NOT_FOUND
            }
            ReviewApiError::Version
            | ReviewApiError::Review(ReviewError::Stale | ReviewError::AnalysisPolicy) => {
                StatusCode::CONFLICT
            }
            ReviewApiError::QueryCapacity
            | ReviewApiError::Review(ReviewError::Busy | ReviewError::PairCapacity) => {
                StatusCode::TOO_MANY_REQUESTS
            }
            ReviewApiError::Diff(DiffError::Limit(_))
            | ReviewApiError::Review(ReviewError::Source(SourceReadError::Capacity)) => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            ReviewApiError::Diff(DiffError::Cancelled) => StatusCode::CONFLICT,
            ReviewApiError::Diff(DiffError::Read { .. }) => StatusCode::SERVICE_UNAVAILABLE,
            ReviewApiError::Diff(_) => StatusCode::UNPROCESSABLE_ENTITY,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        };
        let message = crate::review_application::clip(&self.0.to_string(), 512).0;
        (
            status,
            Json(serde_json::json!({"schema":1,"code":self.0.code(),"message":message})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn occupied_query_or_diff_capacity_refuses_work_without_running_it() {
        let held = QUERIES.acquire_many(2).await.unwrap();
        let result = work(false, || Ok::<_, ReviewApiError>(1)).await;
        assert!(matches!(
            result,
            Err(HttpError(ReviewApiError::QueryCapacity))
        ));
        drop(held);
        let held = DIFFS.acquire().await.unwrap();
        let result = work(true, || Ok::<_, ReviewApiError>(1)).await;
        assert!(matches!(
            result,
            Err(HttpError(ReviewApiError::QueryCapacity))
        ));
        assert_eq!(
            work(false, || Ok::<_, ReviewApiError>(2)).await.unwrap().0,
            2
        );
        drop(held);
        assert_eq!(
            work(true, || Ok::<_, ReviewApiError>(3)).await.unwrap().0,
            3
        );
    }
    #[test]
    fn error_responses_preserve_distinct_recovery_statuses() {
        use crate::review::ReviewError;
        use vizzi_core::{
            comparison::Side,
            model::SourceReadError,
            text_diff::{DiffError, DiffLimit},
        };
        for (error, status) in [
            (ReviewApiError::Invalid("request".into()), 400),
            (ReviewApiError::NotFound, 404),
            (ReviewApiError::Version, 409),
            (ReviewApiError::Review(ReviewError::Stale), 409),
            (ReviewApiError::QueryCapacity, 429),
            (ReviewApiError::Review(ReviewError::PairCapacity), 429),
            (
                ReviewApiError::Diff(DiffError::Limit(DiffLimit::Bytes)),
                413,
            ),
            (
                ReviewApiError::Review(ReviewError::Source(SourceReadError::Capacity)),
                413,
            ),
            (
                ReviewApiError::Diff(DiffError::Read {
                    side: Side::Base,
                    source: SourceReadError::Integrity,
                }),
                503,
            ),
            (ReviewApiError::Diff(DiffError::Cancelled), 409),
            (ReviewApiError::Diff(DiffError::Binary(Side::Working)), 422),
            (ReviewApiError::Closed, 503),
        ] {
            assert_eq!(HttpError(error).into_response().status().as_u16(), status);
        }
    }
}
