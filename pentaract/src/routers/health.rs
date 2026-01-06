use axum::{response::Json, routing::get, Router};
use serde_json::json;

pub struct HealthRouter;

impl HealthRouter {
    pub fn get_router() -> Router {
        Router::new().route("/health", get(health_check))
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
