use serde::{Deserialize, Serialize};

pub const MAX_MESSAGE_LENGTH: usize = 4000;
pub const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;
pub const DEFAULT_MESSAGE_LIMIT: u32 = 64;
pub const MAX_MESSAGE_LIMIT: u32 = 100;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Attachment,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttachmentData {
    pub id: String,
    pub file_name: String,
    pub content_type: String,
    pub url: String,
    pub size: u64,
    pub upload_timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageEdit {
    pub content: Option<String>,
    pub attachment: Option<AttachmentData>,
    pub edited_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub message_id: String,
    pub conversation_id: String,
    pub sender_id: String,
    pub message_type: MessageType,
    pub content: Option<String>,
    pub attachment: Option<AttachmentData>,
    pub created_at: u64,
    pub updated_at: u64,
    pub edit_history: Vec<MessageEdit>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Conversation {
    pub conversation_id: String,
    pub participant_ids: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub last_message_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub content: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct GetMessagesQuery {
    pub limit: Option<u32>,
    pub before: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub sender_id: String,
    pub message_type: MessageType,
    pub content: Option<String>,
    pub attachment: Option<AttachmentData>,
    pub created_at: u64,
    pub updated_at: u64,
    pub is_edited: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConversationResponse {
    pub conversation_id: String,
    pub other_participant_id: String,
    pub created_at: u64,
    pub last_message_at: u64,
}
