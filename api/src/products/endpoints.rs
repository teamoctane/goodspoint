use axum::{
    Json,
    extract::{Extension, Multipart, Path, Query},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde_json::{Value, json};

use super::{
    delegates::{
        add_gallery_items, buy_now_product, create_product, delete_product,
        generate_questions_with_groq, get_gallery, get_product_by_id, get_user_product_by_id,
        is_allowed_content_type, is_allowed_image_type, list_user_products, reorder_gallery,
        replace_gallery, set_product_questions, update_product,
    },
    schemas::{
        BuyNowRequest, CreateProductRequest, DEFAULT_PAGE_LIMIT, GenerateQuestionsPayload,
        GenerateQuestionsRequest, ListMyProductsQuery, MAX_FILE_SIZE, MAX_GALLERY_ITEMS,
        MAX_PAGE_LIMIT, ProductQuestions, ReorderGalleryRequest, UpdateProductRequest,
    },
};
use crate::{
    DB,
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    recommendations::{auto_log_signal, schemas::SignalType},
};
use mongodb::{Collection, bson::doc};

#[inline]
fn strip_embedding_from_product(mut product_value: Value) -> Value {
    if let Some(product_obj) = product_value.as_object_mut() {
        product_obj.remove("embedding");
    }
    product_value
}

pub(crate) async fn create_product_endpoint(
    Extension(user): Extension<UserOut>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut product_data = String::new();
    let mut thumbnail_file: Option<(String, Bytes, String)> = None;
    let mut gallery_files: Vec<(String, Bytes, String)> = Vec::with_capacity(MAX_GALLERY_ITEMS);

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("");

        match field_name {
            "product" => {
                if let Ok(bytes) = field.bytes().await {
                    product_data = String::from_utf8_lossy(&bytes).to_string();
                }
            }
            "thumbnail" => {
                if let Some(file_name) = field.file_name() {
                    let file_name = file_name.to_string();
                    let content_type = field.content_type().unwrap_or("image/jpeg").to_string();
                    if let Ok(bytes) = field.bytes().await {
                        if is_allowed_image_type(&content_type) && bytes.len() <= MAX_FILE_SIZE {
                            thumbnail_file = Some((file_name, bytes, content_type));
                        }
                    }
                }
            }
            "gallery" => {
                if let Some(file_name) = field.file_name() {
                    let file_name = file_name.to_string();
                    let content_type = field
                        .content_type()
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    if let Ok(bytes) = field.bytes().await {
                        if is_allowed_content_type(&content_type) && bytes.len() <= MAX_FILE_SIZE {
                            gallery_files.push((file_name, bytes, content_type));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if product_data.trim().is_empty() {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Product data is required".to_string(),
        )
        .into_response();
    }

    let payload: CreateProductRequest = match serde_json::from_str(&product_data) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid product data: {}", e),
            )
            .into_response();
        }
    };

    match create_product(&user, payload, thumbnail_file, gallery_files).await {
        Ok(product) => {
            let product_json = serde_json::to_value(&product).unwrap();
            let clean_product = strip_embedding_from_product(product_json);

            Json(json!({
                "status": "ok",
                "product": clean_product
            }))
            .into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_product_endpoint(
    Path(product_id): Path<String>,
    headers: axum::http::HeaderMap,
    user: Option<Extension<UserOut>>,
) -> impl IntoResponse {
    match get_product_by_id(&product_id).await {
        Ok(product) => {
            if let Some(Extension(user)) = user {
                auto_log_signal(
                    &user.uid,
                    SignalType::ProductView,
                    product.category.clone(),
                    Some(product_id.clone()),
                    None,
                )
                .await;
            } 
            else if let Some(cookie_header) = headers.get(axum::http::header::COOKIE) {
                if let Ok(cookie_str) = cookie_header.to_str() {
                    let mut auth_cookie = None;
                    for cookie_part in cookie_str.split(';') {
                        let cookie_part = cookie_part.trim();
                        if cookie_part.starts_with("GOODSPOINT_AUTHENTICATION=") {
                            auth_cookie = Some(cookie_part.split('=').nth(1).unwrap_or(""));
                            break;
                        }
                    }
                    
                    if let Some(cookie_value) = auth_cookie {
                        if let Some(database) = DB.get() {
                            let collection: Collection<UserOut> = database.collection("users");
                            let user_result = collection
                                .find_one(doc! {"auth.cookie": cookie_value})
                                .await;
                                
                            if let Ok(Some(user)) = user_result {
                                auto_log_signal(
                                    &user.uid,
                                    SignalType::ProductView,
                                    product.category.clone(),
                                    Some(product_id.clone()),
                                    None,
                                )
                                .await;
                            }
                        }
                    }
                }
            }

            let product_json = serde_json::to_value(&product).unwrap();
            let clean_product = strip_embedding_from_product(product_json);

            Json(json!({
                "status": "ok",
                "product": clean_product
            }))
            .into_response()
        }
        Err(_) => {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Product not found".to_string())
                .into_response()
        }
    }
}

pub(crate) async fn get_user_product_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    match get_user_product_by_id(&user, &product_id).await {
        Ok(product) => {
            auto_log_signal(
                &user.uid,
                SignalType::ProductView,
                product.category.clone(),
                Some(product_id.clone()),
                None,
            )
            .await;

            let product_json = serde_json::to_value(&product).unwrap();
            let clean_product = strip_embedding_from_product(product_json);

            Json(json!({
                "status": "ok",
                "product": clean_product
            }))
            .into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn update_product_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let payload: UpdateProductRequest = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid request format: {}", e),
            )
            .into_response();
        }
    };

    match update_product(&user, &product_id, payload, None).await {
        Ok(product) => {
            let product_json = serde_json::to_value(&product).unwrap();
            let clean_product = strip_embedding_from_product(product_json);

            Json(json!({
                "status": "ok",
                "product": clean_product
            }))
            .into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn delete_product_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    match delete_product(&user, &product_id).await {
        Ok(()) => Json(json!({
            "status": "ok",
            "message": "Product deleted successfully"
        }))
        .into_response(),
        Err(_) => VerboseHTTPError::Standard(
            StatusCode::NOT_FOUND,
            "Failed to delete product".to_string(),
        )
        .into_response(),
    }
}

pub(crate) async fn list_my_products_endpoint(
    Extension(user): Extension<UserOut>,
    Query(params): Query<ListMyProductsQuery>,
) -> impl IntoResponse {
    let limit = params
        .limit
        .unwrap_or(DEFAULT_PAGE_LIMIT)
        .min(MAX_PAGE_LIMIT);
    let offset = params.offset.unwrap_or(0);

    match list_user_products(&user, limit, offset).await {
        Ok(products) => Json(json!({
            "status": "ok",
            "products": products
        }))
        .into_response(),
        Err(_) => VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to retrieve products".to_string(),
        )
        .into_response(),
    }
}

pub(crate) async fn generate_questions_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let payload: GenerateQuestionsPayload = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid request format: {}", e),
            )
            .into_response();
        }
    };

    let request = GenerateQuestionsRequest {
        product_id,
        description: payload.description,
    };

    match generate_questions_with_groq(&user, request).await {
        Ok(questions) => Json(json!({
            "status": "ok",
            "questions": questions
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_gallery_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    match get_gallery(&user, &product_id).await {
        Ok(gallery) => Json(json!({
            "status": "ok",
            "gallery": gallery
        }))
        .into_response(),
        Err(_) => VerboseHTTPError::Standard(
            StatusCode::NOT_FOUND,
            "Failed to retrieve gallery".to_string(),
        )
        .into_response(),
    }
}

pub(crate) async fn replace_gallery_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut gallery_files: Vec<(String, Bytes, String)> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "gallery" {
            if let Some(file_name) = field.file_name() {
                let file_name = file_name.to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                if let Ok(bytes) = field.bytes().await {
                    if is_allowed_content_type(&content_type) && bytes.len() <= MAX_FILE_SIZE {
                        gallery_files.push((file_name, bytes, content_type));
                    }
                }
            }
        }
    }

    if gallery_files.len() > MAX_GALLERY_ITEMS {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Cannot upload more than {} gallery items",
                MAX_GALLERY_ITEMS
            ),
        )
        .into_response();
    }

    match replace_gallery(&user, &product_id, gallery_files).await {
        Ok(gallery) => Json(json!({
            "status": "ok",
            "gallery": gallery
        }))
        .into_response(),
        Err(_) => VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Failed to replace gallery".to_string(),
        )
        .into_response(),
    }
}

pub(crate) async fn add_gallery_items_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut gallery_files: Vec<(String, Bytes, String)> = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "gallery" {
            if let Some(file_name) = field.file_name() {
                let file_name = file_name.to_string();
                let content_type = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_string();
                if let Ok(bytes) = field.bytes().await {
                    if is_allowed_content_type(&content_type) && bytes.len() <= MAX_FILE_SIZE {
                        gallery_files.push((file_name, bytes, content_type));
                    }
                }
            }
        }
    }

    if gallery_files.len() > MAX_GALLERY_ITEMS {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!(
                "Cannot add more than {} gallery items at once",
                MAX_GALLERY_ITEMS
            ),
        )
        .into_response();
    }

    match add_gallery_items(&user, &product_id, gallery_files).await {
        Ok(gallery) => Json(json!({
            "status": "ok",
            "gallery": gallery
        }))
        .into_response(),
        Err(_) => VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Failed to add gallery items".to_string(),
        )
        .into_response(),
    }
}

pub(crate) async fn reorder_gallery_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let payload: ReorderGalleryRequest = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid request format: {}", e),
            )
            .into_response();
        }
    };

    match reorder_gallery(&user, &product_id, payload.item_ids).await {
        Ok(gallery) => Json(json!({
            "status": "ok",
            "gallery": gallery
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_questions_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
) -> impl IntoResponse {
    match get_user_product_by_id(&user, &product_id).await {
        Ok(product) => {
            let questions = product.custom_questions.unwrap_or(ProductQuestions {
                questions: Vec::new(),
            });
            Json(json!({
                "status": "ok",
                "questions": questions
            }))
            .into_response()
        }
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn set_questions_endpoint(
    Extension(user): Extension<UserOut>,
    Path(product_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let questions: ProductQuestions = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid request format: {}", e),
            )
            .into_response();
        }
    };

    match set_product_questions(&user, &product_id, questions).await {
        Ok(updated_questions) => Json(json!({
            "status": "ok",
            "questions": updated_questions
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub async fn buy_now_endpoint(
    Extension(user): Extension<UserOut>,
    Json(request): Json<BuyNowRequest>,
) -> impl IntoResponse {
    match buy_now_product(&user, request.product_id, request.quantity).await {
        Ok(order) => Json(order).into_response(),
        Err(error) => error.into_response(),
    }
}
