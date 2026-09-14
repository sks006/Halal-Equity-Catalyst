use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use equity_catalyst_api::{build_app, config::Config, create_db_pool};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn setup_test_app() -> axum::Router {
    let config = Config::from_env();
    let pool = create_db_pool(&config.database_url).expect("Failed to create test db pool");
    let redis_client = redis::Client::open(config.redis_url.as_str()).ok();
    build_app(config, pool, redis_client)
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = setup_test_app();

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
    let app = setup_test_app();

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
    assert_eq!(json["database"], "healthy");
    assert_eq!(json["redis"], "healthy");
}

#[tokio::test]
async fn test_not_found_fallback() {
    let app = setup_test_app();

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
