use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Json},
};

use super::delegates::*;
use crate::{
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    products::schemas::{ConfirmOrderRequest, ListOrdersQuery},
};

pub async fn list_orders_endpoint(req: Request<Body>) -> impl IntoResponse {
    let Some(user) = req.extensions().get::<UserOut>().cloned() else {
        return VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            .into_response();
    };

    let query = match serde_urlencoded::from_str::<ListOrdersQuery>(req.uri().query().unwrap_or(""))
    {
        Ok(q) => q,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid query parameters".to_string(),
            )
            .into_response();
        }
    };

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    match list_orders(&user, limit, offset).await {
        Ok(orders) => Json(orders).into_response(),
        Err(error) => error.into_response(),
    }
}

pub async fn list_seller_orders_endpoint(req: Request<Body>) -> impl IntoResponse {
    let Some(user) = req.extensions().get::<UserOut>().cloned() else {
        return VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            .into_response();
    };

    let query = match serde_urlencoded::from_str::<ListOrdersQuery>(req.uri().query().unwrap_or(""))
    {
        Ok(q) => q,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Invalid query parameters".to_string(),
            )
            .into_response();
        }
    };

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    match list_seller_orders(&user, limit, offset).await {
        Ok(orders) => Json(orders).into_response(),
        Err(error) => error.into_response(),
    }
}

pub async fn confirm_order_endpoint(req: Request<Body>) -> impl IntoResponse {
    let Some(user) = req.extensions().get::<UserOut>().cloned() else {
        return VerboseHTTPError::Standard(StatusCode::UNAUTHORIZED, "Unauthorized".to_string())
            .into_response();
    };

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                "Failed to read request body".to_string(),
            )
            .into_response();
        }
    };

    let request: ConfirmOrderRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(_) => {
            return VerboseHTTPError::Standard(StatusCode::BAD_REQUEST, "Invalid JSON".to_string())
                .into_response();
        }
    };

    match confirm_order(&user, request.order_id).await {
        Ok(order) => Json(order).into_response(),
        Err(error) => error.into_response(),
    }
}
