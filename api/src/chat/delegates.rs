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
use crate::{
    DB,
    apex::utils::VerboseHTTPError,
    auth::schemas::UserOut,
    products::schemas::ProductCategory,
    recommendations::{auto_log_signal, schemas::SignalType},
};

#[derive(serde::Deserialize)]
struct FilebaseUploadResponse {
    #[serde(rename = "Hash")]
    hash: String,
    #[serde(rename = "Name")]
    _name: String,
    #[serde(rename = "Size")]
    _size: String,
}

#[inline]
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

    let response = reqwest::Client::new()
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

    if !response.status().is_success() {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Filebase upload failed: {}", response.status()),
        ));
    }

    let upload_result: FilebaseUploadResponse = response.json().await.map_err(|_| {
        VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to parse Filebase response".to_string(),
        )
    })?;

    Ok(format!(
        "https://ipfs.filebase.io/ipfs/{}",
        upload_result.hash
    ))
}

pub async fn get_or_create_conversation(
    user_id: &str,
    other_user_id: &str,
) -> Result<String, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let conversations: Collection<Conversation> = database.collection("conversations");

    let mut participant_ids = vec![user_id.to_string(), other_user_id.to_string()];
    participant_ids.sort_unstable();

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
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let conversations: Collection<Conversation> = database.collection("conversations");

    match conversations
        .find_one(doc! {
            "conversation_id": conversation_id,
            "participant_ids": user_id
        })
        .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(VerboseHTTPError::Standard(
            StatusCode::FORBIDDEN,
            "Access denied to this conversation".to_string(),
        )),
        Err(_) => Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database error".to_string(),
        )),
    }
}

pub async fn send_text_message(
    user: &UserOut,
    other_user_id: &str,
    content: &str,
) -> Result<Message, VerboseHTTPError> {
    let content = content.trim();
    if content.is_empty() {
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
        query_data: None,
        quote_data: None,
        created_at: now,
        updated_at: now,
        edit_history: Vec::new(),
    };

    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

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

    log_chat_query_signal(user, content).await;

    send_message_notification(&user.username, other_user_id, MessageType::Text).await;

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
        query_data: None,
        quote_data: None,
        created_at: now,
        updated_at: now,
        edit_history: Vec::new(),
    };

    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

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

    send_message_notification(&user.username, other_user_id, MessageType::Attachment).await;

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

    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

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

    let response_messages = messages_vec
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
    let new_content = new_content.trim();
    if new_content.is_empty() {
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

    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

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
        username: Some(user.username.clone()),
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
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

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

    let response_conversations = conversations_vec
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
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let messages: Collection<Message> = database.collection("messages");
    let users: Collection<crate::auth::schemas::UserOut> = database.collection("users");

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
    
    // Get the sender's username
    let sender = users
        .find_one(doc! { "uid": &message.sender_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?;
    
    let sender_username = sender.map(|u| u.username);
    
    // Add username to all edit history entries
    let mut edit_history = message.edit_history;
    for edit in &mut edit_history {
        edit.username = sender_username.clone();
    }

    Ok(edit_history)
}

pub async fn create_order_from_quote(
    user: &UserOut,
    message_id: String,
) -> Result<crate::products::schemas::Order, VerboseHTTPError> {
    let Some(database) = DB.get() else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Database unavailable".to_string(),
        ));
    };

    let messages: Collection<Message> = database.collection("messages");

    let message = messages
        .find_one(doc! { "message_id": &message_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Quote message not found".to_string())
        })?;

    verify_conversation_access(&message.conversation_id, &user.uid).await?;

    let Some(quote_data) = message.quote_data else {
        return Err(VerboseHTTPError::Standard(
            StatusCode::BAD_REQUEST,
            "Message is not a quote".to_string(),
        ));
    };

    let products: Collection<crate::products::schemas::Product> = database.collection("products");
    let product = products
        .find_one(doc! { "product_id": &quote_data.product_id })
        .await
        .map_err(|_| {
            VerboseHTTPError::Standard(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database error".to_string(),
            )
        })?
        .ok_or_else(|| {
            VerboseHTTPError::Standard(StatusCode::NOT_FOUND, "Product not found".to_string())
        })?;

    let price = quote_data.custom_price.parse::<f64>().map_err(|_| {
        VerboseHTTPError::Standard(StatusCode::BAD_REQUEST, "Invalid price format".to_string())
    })?;

    let order_response = crate::orders::delegates::create_order_internal(
        quote_data.product_id,
        product.user_id,
        user.uid.clone(),
        quote_data.quantity,
        price,
    )
    .await?;

    let order = crate::products::schemas::Order {
        order_id: order_response.order_id,
        product_id: order_response.product_id,
        seller_id: order_response.seller_id,
        buyer_id: order_response.buyer_id,
        quantity: order_response.quantity,
        price: order_response.price,
        status: order_response.status,
        created_at: order_response.created_at,
        updated_at: order_response.updated_at,
    };

    Ok(order)
}

async fn send_message_notification(
    sender_username: &str,
    recipient_user_id: &str,
    message_type: MessageType,
) {
    let Some(database) = DB.get() else {
        return;
    };

    let users: Collection<crate::auth::schemas::UserOut> = database.collection("users");
    let Ok(Some(recipient)) = users.find_one(doc! { "uid": recipient_user_id }).await else {
        return;
    };

    if let Err(_) = recipient.initialize_encryption() {
        return;
    }

    let notification_message = match message_type {
        MessageType::Quote => format!("{} created a quote for you", sender_username),
        MessageType::Query => format!("{} sent you a product inquiry", sender_username),
        _ => format!("{} sent you a message", sender_username),
    };

    let full_message = format!(
        "{} - Check your messages: https://goodspoint.tech/chat",
        notification_message
    );

    let _ = crate::notifications::delegates::send_email_internal(
        &recipient.email.to_string(),
        Some(&recipient.username),
        "New Message - GoodsPoint",
        &full_message,
    )
    .await;

    if recipient.whatsapp_verified {
        if let Some(ref whatsapp) = recipient.whatsapp_number {
            let _ = crate::notifications::delegates::send_whatsapp_internal(
                &whatsapp.to_string(),
                &full_message,
            )
            .await;
        }
    }
}

async fn log_chat_query_signal(user: &UserOut, content: &str) {
    if is_product_query_message(content) {
        let inferred_category = infer_category_from_query(content);
        auto_log_signal(
            &user.uid,
            SignalType::Query,
            inferred_category,
            None,
            Some(content.to_string()),
        )
        .await;


    }
}
fn is_product_query_message(content: &str) -> bool {
    let content_lower = content.to_lowercase();

    let inquiry_keywords = [
        "price",
        "cost",
        "how much",
        "available",
        "stock",
        "buy",
        "purchase",
        "interested",
        "inquiry",
        "quote",
        "details",
        "specs",
        "specification",
        "size",
        "color",
        "delivery",
        "shipping",
        "warranty",
        "condition",
        "discount",
        "offer",
        "deal",
        "negotiable",
    ];

    let question_patterns = [
        "?", "what", "when", "where", "how", "why", "can you", "do you",
    ];

    let has_inquiry = inquiry_keywords
        .iter()
        .any(|&keyword| content_lower.contains(keyword));
    let has_question = question_patterns
        .iter()
        .any(|&pattern| content_lower.contains(pattern));

    has_inquiry || has_question
}

fn infer_category_from_query(query: &str) -> ProductCategory {
    let query_lower = query.to_lowercase();

    if query_lower.contains("phone")
        || query_lower.contains("smartphone")
        || query_lower.contains("mobile")
    {
        ProductCategory::Smartphones
    } else if query_lower.contains("laptop")
        || query_lower.contains("computer")
        || query_lower.contains("pc")
    {
        ProductCategory::Computers
    } else if query_lower.contains("shirt")
        || query_lower.contains("clothing")
        || query_lower.contains("dress")
    {
        ProductCategory::UnisexClothing
    } else if query_lower.contains("shoe")
        || query_lower.contains("sneaker")
        || query_lower.contains("boot")
    {
        ProductCategory::Shoes
    } else if query_lower.contains("kitchen")
        || query_lower.contains("cooking")
        || query_lower.contains("utensil")
    {
        ProductCategory::Kitchen
    } else if query_lower.contains("game")
        || query_lower.contains("gaming")
        || query_lower.contains("console")
    {
        ProductCategory::Gaming
    } else if query_lower.contains("car")
        || query_lower.contains("auto")
        || query_lower.contains("vehicle")
    {
        ProductCategory::CarParts
    } else if query_lower.contains("beauty")
        || query_lower.contains("makeup")
        || query_lower.contains("cosmetic")
    {
        ProductCategory::Beauty
    } else if query_lower.contains("book")
        || query_lower.contains("reading")
        || query_lower.contains("novel")
    {
        ProductCategory::Books
    } else if query_lower.contains("toy") || query_lower.contains("plaything") {
        ProductCategory::Toys
    } else if query_lower.contains("fitness")
        || query_lower.contains("exercise")
        || query_lower.contains("workout")
    {
        ProductCategory::FitnessEquipment
    } else if query_lower.contains("furniture")
        || query_lower.contains("chair")
        || query_lower.contains("table")
    {
        ProductCategory::Furniture
    } else if query_lower.contains("jewelry")
        || query_lower.contains("necklace")
        || query_lower.contains("ring")
    {
        ProductCategory::Jewelry
    } else if query_lower.contains("bag")
        || query_lower.contains("purse")
        || query_lower.contains("backpack")
    {
        ProductCategory::Bags
    } else if query_lower.contains("tool") || query_lower.contains("hardware") {
        ProductCategory::HomeTools
    } else {
        ProductCategory::UnisexClothing
    }
}
