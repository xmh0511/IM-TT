use salvo::prelude::*;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use serde::{Deserialize, Serialize};
use futures_util::{StreamExt, SinkExt};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::entity::{messages, group_members, group_members::Entity as GroupMembers};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event_type: String,
    pub user_id: i64,
    pub receiver_id: Option<i64>,
    pub group_id: Option<i64>,
    pub content: Option<String>,
    pub data: Option<serde_json::Value>,
}

pub type Clients = Arc<Mutex<HashMap<i64, mpsc::UnboundedSender<String>>>>;

pub fn create_clients() -> Clients {
    Arc::new(Mutex::new(HashMap::new()))
}

#[handler]
pub async fn websocket_handler(
    req: &mut Request,
    res: &mut Response,
    depot: &mut Depot,
) -> Result<(), salvo::http::StatusError> {
    let user_id = depot.get::<i64>("user_id").ok();
    let clients = depot.get::<Clients>("clients").ok();
    let db = depot.get::<DatabaseConnection>("db").ok().cloned();

    if user_id.is_none() || clients.is_none() || db.is_none() {
        return Err(salvo::http::StatusError::unauthorized());
    }

    let user_id = *user_id.unwrap();
    let clients = clients.unwrap().clone();
    let db = db.unwrap();

    tracing::info!("WS upgrade request from user {}", user_id);

    WebSocketUpgrade::new()
        .upgrade(req, res, move |ws| handle_socket(ws, user_id, clients, db))
        .await
}

async fn handle_socket(ws: WebSocket, user_id: i64, clients: Clients, db: DatabaseConnection) {
    tracing::info!("WS connected: user {}", user_id);

    // Set user status to online in database
    {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};
        use crate::entity::{users, users::Entity as Users};
        if let Ok(Some(user)) = Users::find_by_id(user_id).one(&db).await {
            let mut active: users::ActiveModel = user.into();
            active.status = Set("online".to_string());
            let _ = active.update(&db).await;
        }
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    {
        let mut clients_lock = clients.lock().await;
        clients_lock.insert(user_id, tx);
    }

    // Send currently online users to this user, and broadcast this user's online status
    {
        let clients_lock = clients.lock().await;

        // Tell this user who is already online (send to this user's channel)
        if let Some(my_sender) = clients_lock.get(&user_id) {
            for (uid, _) in clients_lock.iter() {
                if *uid != user_id {
                    let online_event = WsEvent {
                        event_type: "online".to_string(),
                        user_id: *uid,
                        receiver_id: None,
                        group_id: None,
                        content: None,
                        data: None,
                    };
                    let _ = my_sender.send(serde_json::to_string(&online_event).unwrap());
                }
            }
        }

        // Tell other users this user is now online
        let online_event = WsEvent {
            event_type: "online".to_string(),
            user_id,
            receiver_id: None,
            group_id: None,
            content: None,
            data: None,
        };
        for (uid, sender) in clients_lock.iter() {
            if *uid != user_id {
                let _ = sender.send(serde_json::to_string(&online_event).unwrap());
            }
        }
    }

    let (mut sink, mut stream) = ws.split();

    loop {
        tokio::select! {
            ws_msg = stream.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_text() {
                            if let Ok(text) = msg.as_str() {
                                if let Ok(event) = serde_json::from_str::<WsEvent>(text) {
                                    tracing::debug!("WS event from {}: {}", user_id, event.event_type);
                                    match event.event_type.as_str() {
                                        "message" => {
                                            if let Some(content) = &event.content {
                                                let _ = save_message(
                                                    &db, user_id, event.receiver_id, event.group_id,
                                                    content, "text",
                                                ).await;
                                            }

                                            let clients_lock = clients.lock().await;
                                            if let Some(receiver_id) = event.receiver_id {
                                                if let Some(sender) = clients_lock.get(&receiver_id) {
                                                    let _ = sender.send(serde_json::to_string(&event).unwrap());
                                                }
                                            } else if let Some(group_id) = event.group_id {
                                                if let Ok(members) = GroupMembers::find()
                                                    .filter(group_members::Column::GroupId.eq(group_id))
                                                    .all(&db).await
                                                {
                                                    for member in members {
                                                        if member.user_id != user_id {
                                                            if let Some(sender) = clients_lock.get(&member.user_id) {
                                                                let _ = sender.send(serde_json::to_string(&event).unwrap());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        "typing" => {
                                            if let Some(receiver_id) = event.receiver_id {
                                                let clients_lock = clients.lock().await;
                                                if let Some(sender) = clients_lock.get(&receiver_id) {
                                                    let _ = sender.send(serde_json::to_string(&event).unwrap());
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        } else if msg.is_ping() {
                            let _ = sink.send(WsMessage::pong(msg.as_bytes().to_vec())).await;
                        } else if msg.is_close() {
                            tracing::info!("WS close from user {}", user_id);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WS error for user {}: {:?}", user_id, e);
                        break;
                    }
                    None => {
                        tracing::info!("WS stream ended for user {}", user_id);
                        break;
                    }
                }
            }
            Some(msg) = rx.recv() => {
                if let Err(e) = sink.send(WsMessage::text(msg)).await {
                    tracing::error!("Failed to send to user {}: {:?}", user_id, e);
                    break;
                }
            }
            else => {
                tracing::info!("Both channels closed for user {}", user_id);
                break;
            }
        }
    }

    // Broadcast offline status
    {
        let offline_event = WsEvent {
            event_type: "offline".to_string(),
            user_id,
            receiver_id: None,
            group_id: None,
            content: None,
            data: None,
        };
        let clients_lock = clients.lock().await;
        for (uid, sender) in clients_lock.iter() {
            if *uid != user_id {
                let _ = sender.send(serde_json::to_string(&offline_event).unwrap());
            }
        }
    }

    // Set user status to offline in database
    {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};
        use crate::entity::{users, users::Entity as Users};
        if let Ok(Some(user)) = Users::find_by_id(user_id).one(&db).await {
            let mut active: users::ActiveModel = user.into();
            active.status = Set("offline".to_string());
            let _ = active.update(&db).await;
        }
    }

    let mut clients_lock = clients.lock().await;
    clients_lock.remove(&user_id);
    tracing::info!("WS cleanup done for user {}", user_id);
}

async fn save_message(
    db: &DatabaseConnection,
    sender_id: i64,
    receiver_id: Option<i64>,
    group_id: Option<i64>,
    content: &str,
    message_type: &str,
) -> Result<messages::Model, sea_orm::DbErr> {
    let new_message = messages::ActiveModel {
        sender_id: Set(sender_id),
        receiver_id: Set(receiver_id),
        group_id: Set(group_id),
        content: Set(content.to_string()),
        message_type: Set(message_type.to_string()),
        is_read: Set(false),
        ..Default::default()
    };

    new_message.insert(db).await
}
