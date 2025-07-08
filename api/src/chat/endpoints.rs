use axum::{
    Json,
    extract::{Extension, Multipart, Path, Query},
    http::StatusCode,
    response::IntoResponse,
};
use bytes::Bytes;
use serde_json::json;

use super::{
    delegates::{
        edit_message, get_message_edit_history, get_messages, get_user_conversations,
        is_allowed_attachment_type, send_attachment_message, send_text_message,
    },
    schemas::{
        DEFAULT_MESSAGE_LIMIT, EditMessageRequest, GetMessagesQuery, MAX_FILE_SIZE,
        MAX_MESSAGE_LIMIT,
    },
};
use crate::{apex::utils::VerboseHTTPError, auth::schemas::UserOut};

pub(crate) async fn send_message_endpoint(
    Extension(user): Extension<UserOut>,
    Path(other_user_id): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut text_content: Option<String> = None;
    let mut attachment_file: Option<(String, Bytes, String)> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let Some(field_name) = field.name() else {
            continue;
        };

        match field_name {
            "content" => {
                if let Ok(bytes) = field.bytes().await {
                    text_content = Some(String::from_utf8_lossy(&bytes).to_string());
                }
            }
            "attachment" => {
                if let Some(file_name) = field.file_name() {
                    let file_name = file_name.to_string();
                    let content_type = field
                        .content_type()
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    if let Ok(bytes) = field.bytes().await {
                        if is_allowed_attachment_type(&content_type) && bytes.len() <= MAX_FILE_SIZE
                        {
                            attachment_file = Some((file_name, bytes, content_type));
                        } else {
                            return VerboseHTTPError::Standard(
                                StatusCode::BAD_REQUEST,
                                "Invalid file type or size".to_string(),
                            )
                            .into_response();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if text_content.is_none() && attachment_file.is_none() {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Message must contain either text content or an attachment".to_string(),
        )
        .into_response();
    }

    if text_content.is_some() && attachment_file.is_some() {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Message cannot contain both text and attachment".to_string(),
        )
        .into_response();
    }

    let message_result = if let Some(content) = text_content {
        send_text_message(&user, &other_user_id, &content).await
    } else if let Some((file_name, file_data, content_type)) = attachment_file {
        send_attachment_message(&user, &other_user_id, file_name, file_data, content_type).await
    } else {
        return VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Invalid message data".to_string(),
        )
        .into_response();
    };

    match message_result {
        Ok(message) => Json(json!({
            "status": "ok",
            "message": {
                "message_id": message.message_id,
                "sender_id": message.sender_id,
                "message_type": message.message_type,
                "content": message.content,
                "attachment": message.attachment,
                "created_at": message.created_at,
                "updated_at": message.updated_at,
                "is_edited": false
            }
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_messages_endpoint(
    Extension(user): Extension<UserOut>,
    Path(other_user_id): Path<String>,
    Query(params): Query<GetMessagesQuery>,
) -> impl IntoResponse {
    let limit = params
        .limit
        .unwrap_or(DEFAULT_MESSAGE_LIMIT)
        .min(MAX_MESSAGE_LIMIT);

    match get_messages(&user, &other_user_id, limit, params.before.as_deref()).await {
        Ok(messages) => Json(json!({
            "status": "ok",
            "messages": messages
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn edit_message_endpoint(
    Extension(user): Extension<UserOut>,
    Path(message_id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let payload: EditMessageRequest = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            return VerboseHTTPError::Standard(
                StatusCode::BAD_REQUEST,
                format!("Invalid request format: {}", e),
            )
            .into_response();
        }
    };

    match edit_message(&user, &message_id, &payload.content).await {
        Ok(message) => Json(json!({
            "status": "ok",
            "message": {
                "message_id": message.message_id,
                "sender_id": message.sender_id,
                "message_type": message.message_type,
                "content": message.content,
                "attachment": message.attachment,
                "created_at": message.created_at,
                "updated_at": message.updated_at,
                "is_edited": !message.edit_history.is_empty()
            }
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_conversations_endpoint(
    Extension(user): Extension<UserOut>,
) -> impl IntoResponse {
    match get_user_conversations(&user).await {
        Ok(conversations) => Json(json!({
            "status": "ok",
            "conversations": conversations
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn get_message_history_endpoint(
    Extension(user): Extension<UserOut>,
    Path(message_id): Path<String>,
) -> impl IntoResponse {
    match get_message_edit_history(&user, &message_id).await {
        Ok(edit_history) => Json(json!({
            "status": "ok",
            "edit_history": edit_history
        }))
        .into_response(),
        Err(err) => err.into_response(),
    }
}

pub(crate) async fn create_order_from_quote_endpoint(
    Extension(user): Extension<UserOut>,
    Json(request): Json<crate::products::schemas::CreateOrderFromQuoteRequest>,
) -> impl IntoResponse {
    match super::delegates::create_order_from_quote(&user, request.message_id).await {
        Ok(order) => Json(order).into_response(),
        Err(error) => error.into_response(),
    }
}
