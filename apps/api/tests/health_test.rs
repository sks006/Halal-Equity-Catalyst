use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use equity_catalyst_api::{build_app, config::Config};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn test_health_endpoint() {
    let app = build_app(Config::default());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["cluster"], "devnet");
    assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    assert!(json["uptime_seconds"].as_u64().is_some());
}

#[tokio::test]
async fn test_ready_endpoint() {
    let app = build_app(Config::default());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["ready"], true);
}

#[tokio::test]
async fn test_not_found_fallback() {
    let app = build_app(Config::default());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/unknown_route")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("/api/unknown_route"));
}
