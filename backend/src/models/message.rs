use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub receiver_id: Option<i64>,
    pub group_id: Option<i64>,
    pub content: String,
    pub message_type: String,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: crate::entity::messages::Model,
}

