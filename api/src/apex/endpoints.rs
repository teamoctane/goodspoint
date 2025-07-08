use axum::Json;
use serde_json::json;

pub async fn root_endpoint() -> Json<serde_json::Value> {
    Json(json!({
        "message": "ok"
    }))
}
