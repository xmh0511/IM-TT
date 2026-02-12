use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Message {
    pub id: i64,
    pub sender_id: i64,
    pub receiver_id: Option<i64>, // null for group messages
    pub group_id: Option<i64>,
    pub content: String,
    pub message_type: String, // text, image, file
    pub created_at: DateTime<Utc>,
    pub is_read: bool,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub receiver_id: Option<i64>,
    pub group_id: Option<i64>,
    pub content: String,
    pub message_type: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: Message,
}
