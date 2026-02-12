use salvo::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait, QueryOrder};
use crate::models::SendMessageRequest;
use crate::entity::{messages, messages::Entity as Messages};

#[handler]
pub async fn send_message(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let message_data = match req.parse_json::<SendMessageRequest>().await {
        Ok(data) => data,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "Invalid request data"
            })));
            return;
        }
    };

    // Validate that either receiver_id or group_id is provided
    if message_data.receiver_id.is_none() && message_data.group_id.is_none() {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(serde_json::json!({
            "error": "Either receiver_id or group_id must be provided"
        })));
        return;
    }

    let new_message = messages::ActiveModel {
        sender_id: Set(*user_id),
        receiver_id: Set(message_data.receiver_id),
        group_id: Set(message_data.group_id),
        content: Set(message_data.content),
        message_type: Set(message_data.message_type),
        is_read: Set(false),
        ..Default::default()
    };

    match new_message.insert(db).await {
        Ok(message) => {
            res.render(Json(message));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to send message"
            })));
        }
    }
}

#[handler]
pub async fn get_messages(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let receiver_id: Option<i64> = req.query("receiver_id");
    let group_id: Option<i64> = req.query("group_id");
    
    let messages_result = if let Some(receiver_id) = receiver_id {
        // Get personal chat messages
        Messages::find()
            .filter(
                messages::Column::SenderId.eq(*user_id)
                    .and(messages::Column::ReceiverId.eq(receiver_id))
                    .or(
                        messages::Column::SenderId.eq(receiver_id)
                            .and(messages::Column::ReceiverId.eq(*user_id))
                    )
            )
            .order_by_asc(messages::Column::CreatedAt)
            .all(db)
            .await
    } else if let Some(group_id) = group_id {
        // Get group chat messages
        Messages::find()
            .filter(messages::Column::GroupId.eq(group_id))
            .order_by_asc(messages::Column::CreatedAt)
            .all(db)
            .await
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(serde_json::json!({
            "error": "Either receiver_id or group_id must be provided"
        })));
        return;
    };

    match messages_result {
        Ok(messages) => {
            res.render(Json(messages));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to fetch messages"
            })));
        }
    }
}

#[handler]
pub async fn mark_as_read(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();
    
    let message_id: i64 = match req.param::<String>("id") {
        Some(id) => id.parse().unwrap_or(0),
        None => 0,
    };

    let message = Messages::find_by_id(message_id).one(db).await;

    match message {
        Ok(Some(msg)) if msg.receiver_id == Some(*user_id) => {
            let mut message_active: messages::ActiveModel = msg.into();
            message_active.is_read = Set(true);
            
            match message_active.update(db).await {
                Ok(_) => {
                    res.render(Json(serde_json::json!({
                        "success": true
                    })));
                }
                Err(_) => {
                    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                    res.render(Json(serde_json::json!({
                        "error": "Failed to mark message as read"
                    })));
                }
            }
        }
        _ => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(serde_json::json!({
                "error": "Message not found"
            })));
        }
    }
}
