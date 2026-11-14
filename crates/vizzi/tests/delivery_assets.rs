//! Browser delivery uses native-hosted modules without widening embedding policy.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tower::ServiceExt;
use vizzi::application::Application;

#[tokio::test]
async fn confirmed_delivery_assets_are_served_under_the_existing_frame_policy() {
    let app = Arc::new(Application::new("delivery fixture".into()).unwrap());
    let router = vizzi::server::router(app);
    for (uri, content_type, export) in [
        ("/present", "text/html", "/delivery-landing.js"),
        (
            "/workspace-delivery.mjs",
            "text/javascript",
            "createDeliveryClient",
        ),
        (
            "/editor-delivery.js",
            "text/javascript",
            "installWorkspaceDelivery",
        ),
        (
            "/call-delivery.js",
            "text/javascript",
            "createCallPreparation",
        ),
        (
            "/review-delivery.js",
            "text/javascript",
            "createReviewPreparation",
        ),
        ("/delivery-landing.js", "text/javascript", "landing"),
    ] {
        let response = router
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with(content_type)
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let policy = response.headers()["content-security-policy"]
            .to_str()
            .unwrap();
        assert!(policy.contains("frame-ancestors vscode-webview: vscode-file:"));
        assert!(!policy.contains('*'));
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(
            std::str::from_utf8(&bytes).unwrap().contains(export),
            "{uri}"
        );
    }
}
