use axum::http::StatusCode;
use bytes::Bytes;
use futures::TryStreamExt;
use mongodb::{Collection, bson::doc, options::FindOptions};
use reqwest::multipart::{Form, Part};
use std::{
    env::var,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

use super::schemas::*;
use crate::{DB, apex::utils::VerboseHTTPError, auth::schemas::UserOut};

#[derive(serde::Deserialize)]
struct FilebaseUploadResponse {
    #[serde(rename = "Hash")]
    hash: String,
    #[serde(rename = "Name")]
    _name: String,
    #[serde(rename = "Size")]
    _size: String,
}

pub fn is_allowed_attachment_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/gif"
            | "image/webp"
            | "video/mp4"
            | "video/quicktime"
            | "video/x-msvideo"
            | "text/plain"
            | "application/octet-stream"
    )
}

pub async fn upload_file_to_filebase(
    file_name: &str,
    file_data: Bytes,
    content_type: &str,
) -> Result<String, VerboseHTTPError> {
    let ipfs_endpoint =
        var("FILEBASE_IPFS_ENDPOINT").unwrap_or_else(|_| "https://api.filebase.io".to_string());
    let access_key = var("FILEBASE_ACCESS_KEY").expect("FILEBASE_ACCESS_KEY must be set");

    let file_part = Part::bytes(file_data.to_vec())
        .file_name(file_name.to_string())
        .mime_str(content_type)
        .unwrap();

    let form = Form::new().part("file", file_part);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/v0/add?pin=true", ipfs_endpoint))
        .header("Authorization", format!("Bearer {}", access_key))
        .multipart(form)
        .send()
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to upload to Filebase IPFS".to_string(),
            )
        })?;

    let status = response.status();

    if !status.is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Filebase upload failed: {}", status),
        ));
    }

    let upload_result: FilebaseUploadResponse = response.json().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse Filebase response".to_string(),
        )
    })?;

    let file_url = format!("https://ipfs.filebase.io/ipfs/{}", upload_result.hash);
    Ok(file_url)
}

pub async fn get_or_create_conversation(
    user_id: &str,
    other_user_id: &str,
) -> Result<String, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let conversations: Collection<Conversation> = database.collection("conversations");

    let mut participant_ids = vec![user_id.to_string(), other_user_id.to_string()];
    participant_ids.sort();

    if let Ok(Some(conversation)) = conversations
        .find_one(doc! {
            "participant_ids": { "$all": &participant_ids, "$size": 2 }
        })
        .await
    {
        return Ok(conversation.conversation_id);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let conversation = Conversation {
        conversation_id: Uuid::new_v4().to_string(),
        participant_ids,
        created_at: now,
        updated_at: now,
        last_message_at: now,
    };

    conversations.insert_one(&conversation).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to create conversation".to_string(),
        )
    })?;

    Ok(conversation.conversation_id)
}

pub async fn verify_conversation_access(
    conversation_id: &str,
    user_id: &str,
) -> Result<(), VerboseHTTPError> {
    let database = DB.get().unwrap();
    let conversations: Collection<Conversation> = database.collection("conversations");

    if conversations
        .find_one(doc! {
            "conversation_id": conversation_id,
            "participant_ids": user_id
        })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .is_some()
    {
        Ok(())
    } else {
        Err(VerboseHTTPError::Standard(
            StatusCode::FORBIDDEN,
            "Access denied to this conversation".to_string(),
        ))
    }
}

pub async fn send_text_message(
    user: &UserOut,
    other_user_id: &str,
    content: &str,
) -> Result<Message, VerboseHTTPError> {
    if content.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Message content cannot be empty".to_string(),
        ));
    }

    if content.len() > MAX_MESSAGE_LENGTH {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!("Message cannot exceed {} characters", MAX_MESSAGE_LENGTH),
        ));
    }

    let conversation_id = get_or_create_conversation(&user.uid, other_user_id).await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let message = Message {
        message_id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        sender_id: user.uid.clone(),
        message_type: MessageType::Text,
        content: Some(content.to_string()),
        attachment: None,
        created_at: now,
        updated_at: now,
        edit_history: Vec::new(),
    };

    let database = DB.get().unwrap();
    let messages: Collection<Message> = database.collection("messages");
    let conversations: Collection<Conversation> = database.collection("conversations");

    messages.insert_one(&message).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send message".to_string(),
        )
    })?;

    conversations
        .update_one(
            doc! { "conversation_id": &conversation_id },
            doc! {
                "$set": {
                    "updated_at": now as i64,
                    "last_message_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update conversation".to_string(),
            )
        })?;

    Ok(message)
}

pub async fn send_attachment_message(
    user: &UserOut,
    other_user_id: &str,
    file_name: String,
    file_data: Bytes,
    content_type: String,
) -> Result<Message, VerboseHTTPError> {
    if !is_allowed_attachment_type(&content_type) {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "File type not allowed".to_string(),
        ));
    }

    if file_data.len() > MAX_FILE_SIZE {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!("File size cannot exceed {} bytes", MAX_FILE_SIZE),
        ));
    }

    let file_url = upload_file_to_filebase(&file_name, file_data.clone(), &content_type).await?;

    let conversation_id = get_or_create_conversation(&user.uid, other_user_id).await?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let attachment = AttachmentData {
        id: Uuid::new_v4().to_string(),
        file_name,
        content_type,
        url: file_url,
        size: file_data.len() as u64,
        upload_timestamp: now,
    };

    let message = Message {
        message_id: Uuid::new_v4().to_string(),
        conversation_id: conversation_id.clone(),
        sender_id: user.uid.clone(),
        message_type: MessageType::Attachment,
        content: None,
        attachment: Some(attachment),
        created_at: now,
        updated_at: now,
        edit_history: Vec::new(),
    };

    let database = DB.get().unwrap();
    let messages: Collection<Message> = database.collection("messages");
    let conversations: Collection<Conversation> = database.collection("conversations");

    messages.insert_one(&message).await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to send message".to_string(),
        )
    })?;

    conversations
        .update_one(
            doc! { "conversation_id": &conversation_id },
            doc! {
                "$set": {
                    "updated_at": now as i64,
                    "last_message_at": now as i64
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to update conversation".to_string(),
            )
        })?;

    Ok(message)
}

pub async fn get_messages(
    user: &UserOut,
    other_user_id: &str,
    limit: u32,
    before: Option<&str>,
) -> Result<Vec<MessageResponse>, VerboseHTTPError> {
    let conversation_id = get_or_create_conversation(&user.uid, other_user_id).await?;
    verify_conversation_access(&conversation_id, &user.uid).await?;

    let database = DB.get().unwrap();
    let messages: Collection<Message> = database.collection("messages");

    let mut filter = doc! { "conversation_id": &conversation_id };

    if let Some(before_id) = before {
        if let Ok(Some(before_message)) = messages.find_one(doc! { "message_id": before_id }).await
        {
            filter.insert(
                "created_at",
                doc! { "$lt": before_message.created_at as i64 },
            );
        }
    }

    let find_options = FindOptions::builder()
        .sort(doc! { "created_at": -1 })
        .limit(limit as i64)
        .build();

    let cursor = messages
        .find(filter)
        .with_options(find_options)
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve messages".to_string(),
            )
        })?;

    let messages_vec: Vec<Message> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to collect messages".to_string(),
        )
    })?;

    let response_messages: Vec<MessageResponse> = messages_vec
        .into_iter()
        .rev()
        .map(|msg| MessageResponse {
            message_id: msg.message_id,
            sender_id: msg.sender_id,
            message_type: msg.message_type,
            content: msg.content,
            attachment: msg.attachment,
            created_at: msg.created_at,
            updated_at: msg.updated_at,
            is_edited: !msg.edit_history.is_empty(),
        })
        .collect();

    Ok(response_messages)
}

pub async fn edit_message(
    user: &UserOut,
    message_id: &str,
    new_content: &str,
) -> Result<Message, VerboseHTTPError> {
    if new_content.trim().is_empty() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Message content cannot be empty".to_string(),
        ));
    }

    if new_content.len() > MAX_MESSAGE_LENGTH {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            format!("Message cannot exceed {} characters", MAX_MESSAGE_LENGTH),
        ));
    }

    let database = DB.get().unwrap();
    let messages: Collection<Message> = database.collection("messages");

    let message = messages
        .find_one(doc! {
            "message_id": message_id,
            "sender_id": &user.uid
        })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(
                StatusCode::NOT_FOUND,
                "Message not found or access denied".to_string(),
            )
        })?;

    if message.message_type != MessageType::Text {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Can only edit text messages".to_string(),
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let edit_entry = MessageEdit {
        content: message.content.clone(),
        attachment: message.attachment.clone(),
        edited_at: now,
    };

    let updated_message = messages
        .find_one_and_update(
            doc! { "message_id": message_id },
            doc! {
                "$set": {
                    "content": new_content,
                    "updated_at": now as i64
                },
                "$push": {
                    "edit_history": mongodb::bson::to_bson(&edit_entry).unwrap()
                }
            },
        )
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to edit message".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Message not found".to_string())
        })?;

    Ok(updated_message)
}

pub async fn get_user_conversations(
    user: &UserOut,
) -> Result<Vec<ConversationResponse>, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let conversations: Collection<Conversation> = database.collection("conversations");

    let cursor = conversations
        .find(doc! { "participant_ids": &user.uid })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve conversations".to_string(),
            )
        })?;

    let conversations_vec: Vec<Conversation> = cursor.try_collect().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to collect conversations".to_string(),
        )
    })?;

    let response_conversations: Vec<ConversationResponse> = conversations_vec
        .into_iter()
        .map(|conv| {
            let other_participant_id = conv
                .participant_ids
                .iter()
                .find(|&id| id != &user.uid)
                .unwrap_or(&user.uid)
                .clone();

            ConversationResponse {
                conversation_id: conv.conversation_id,
                other_participant_id,
                created_at: conv.created_at,
                last_message_at: conv.last_message_at,
            }
        })
        .collect();

    Ok(response_conversations)
}

pub async fn get_message_edit_history(
    user: &UserOut,
    message_id: &str,
) -> Result<Vec<MessageEdit>, VerboseHTTPError> {
    let database = DB.get().unwrap();
    let messages: Collection<Message> = database.collection("messages");

    let message = messages
        .find_one(doc! { "message_id": message_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Message not found".to_string())
        })?;

    verify_conversation_access(&message.conversation_id, &user.uid).await?;

    Ok(message.edit_history)
}
