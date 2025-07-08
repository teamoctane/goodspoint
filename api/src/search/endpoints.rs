use axum::{
    Json,
    extract::{Extension, Multipart, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use super::{
    delegates::{optimized_search_products},
    schemas::{
        MAX_IMAGE_SIZE, MAX_IMAGES_PER_REQUEST,
        SimpleSearchRequest,
    },
};
use crate::{
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    recommendations::{auto_log_signal, schemas::SignalType},
};

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub force_original: Option<bool>,
}

pub async fn optimized_search_products_endpoint(
    Query(params): Query<SearchQueryParams>,
    user: Option<Extension<UserOut>>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut request = SimpleSearchRequest {
        query: None,
        limit: None,
        force_original: params.force_original,
    };
    let mut image_files = Vec::with_capacity(MAX_IMAGES_PER_REQUEST);
    let mut image_count = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "body" => {
                if let Ok(data) = field.bytes().await {
                    if let Ok(mut json_request) =
                        serde_json::from_slice::<SimpleSearchRequest>(&data)
                    {
                        if json_request.force_original.is_none() {
                            json_request.force_original = params.force_original;
                        }
                        request = json_request;
                    } else {
                        return VerboseHTTPError::Standard(
                            StatusCode::BAD_REQUEST,
                            "Invalid JSON in body field".to_string(),
                        ).into_response();
                    }
                }
            }
            "images" => {
                if image_count >= MAX_IMAGES_PER_REQUEST {
                    return VerboseHTTPError::Standard(
                        StatusCode::BAD_REQUEST,
                        "Maximum 2 images allowed per search request".to_string(),
                    ).into_response();
                }

                let filename = field.file_name().unwrap_or("image").to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();

                if !content_type.starts_with("image/") {
                    return VerboseHTTPError::Standard(
                        StatusCode::BAD_REQUEST,
                        format!("File '{}' is not a valid image", filename),
                    ).into_response();
                }

                if let Ok(data) = field.bytes().await {
                    if data.len() > MAX_IMAGE_SIZE {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(serde_json::json!({
                                "error": format!("Image '{}' exceeds 5MB size limit", filename)
                            })),
                        )
                            .into_response();
                    }

                    image_files.push((filename, data, content_type));
                    image_count += 1;
                } else {
                    return VerboseHTTPError::Standard(
                        StatusCode::BAD_REQUEST,
                        format!("Failed to read image data for '{}'", filename),
                    ).into_response();
                }
            }
            _ => {}
        }
    }

    let original_query = request.query.clone();

    match optimized_search_products(request, image_files).await {
        Ok(response) => {                if let Some(Extension(user)) = user {
                if let Some(ref query) = response.enhanced_query {
                    auto_log_signal(
                        &user.uid,
                        SignalType::Search,
                        response
                            .inferred_category
                            .unwrap_or(crate::products::schemas::ProductCategory::Other),
                        None,
                        Some(query.clone()),
                    )
                    .await;
                } else if let Some(ref orig_query) = original_query {
                    auto_log_signal(
                        &user.uid,
                        SignalType::Search,
                        crate::products::schemas::ProductCategory::Other,
                        None,
                        Some(orig_query.clone()),
                    )
                    .await;
                }
            }

            Json(response).into_response()
        }
        Err(error) => error.into_response(),
    }
}
