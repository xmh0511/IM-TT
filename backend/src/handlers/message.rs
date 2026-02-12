use salvo::prelude::*;
use sqlx::MySqlPool;
use crate::models::{SendMessageRequest, Message};

#[handler]
pub async fn send_message(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.obtain::<i64>().unwrap();

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

    let result = sqlx::query(
        "INSERT INTO messages (sender_id, receiver_id, group_id, content, message_type) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(message_data.receiver_id)
    .bind(message_data.group_id)
    .bind(&message_data.content)
    .bind(&message_data.message_type)
    .execute(pool)
    .await;

    match result {
        Ok(query_result) => {
            let message_id = query_result.last_insert_id() as i64;
            
            let message = sqlx::query_as::<_, Message>("SELECT * FROM messages WHERE id = ?")
                .bind(message_id)
                .fetch_one(pool)
                .await
                .unwrap();

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
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.obtain::<i64>().unwrap();

    let receiver_id: Option<i64> = req.query("receiver_id");
    let group_id: Option<i64> = req.query("group_id");
    
    let messages = if let Some(receiver_id) = receiver_id {
        // Get personal chat messages
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE (sender_id = ? AND receiver_id = ?) OR (sender_id = ? AND receiver_id = ?) ORDER BY created_at ASC"
        )
        .bind(user_id)
        .bind(receiver_id)
        .bind(receiver_id)
        .bind(user_id)
        .fetch_all(pool)
        .await
    } else if let Some(group_id) = group_id {
        // Get group chat messages
        sqlx::query_as::<_, Message>(
            "SELECT * FROM messages WHERE group_id = ? ORDER BY created_at ASC"
        )
        .bind(group_id)
        .fetch_all(pool)
        .await
    } else {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(serde_json::json!({
            "error": "Either receiver_id or group_id must be provided"
        })));
        return;
    };

    match messages {
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
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.obtain::<i64>().unwrap();
    
    let message_id: i64 = match req.param("id") {
        Some(id) => id.parse().unwrap_or(0),
        None => 0,
    };

    let result = sqlx::query(
        "UPDATE messages SET is_read = TRUE WHERE id = ? AND receiver_id = ?"
    )
    .bind(message_id)
    .bind(user_id)
    .execute(pool)
    .await;

    match result {
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
