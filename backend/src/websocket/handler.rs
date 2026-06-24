use salvo::prelude::*;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use std::collections::{HashMap, HashSet};
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

#[derive(Debug)]
pub struct ClientEntry {
    pub sender: mpsc::UnboundedSender<String>,
    pub user_id: i64,
}

pub type Clients = Arc<Mutex<HashMap<String, ClientEntry>>>;

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
    let conn_id = format!("{}_{}", user_id, uuid::Uuid::new_v4());
    tracing::info!("WS connected: user {} conn {}", user_id, conn_id);

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register this connection and check if it's the first for this user
    let is_first_connection = {
        let mut clients_lock = clients.lock().await;
        let first = !clients_lock.values().any(|e| e.user_id == user_id);
        clients_lock.insert(conn_id.clone(), ClientEntry { sender: tx, user_id });
        first
    };

    // Broadcast online status only for the first connection
    if is_first_connection {
        let clients_lock = clients.lock().await;

        // Tell this user who is already online
        if let Some(my_entry) = clients_lock.get(&conn_id) {
            let mut seen_users: HashSet<i64> = HashSet::new();
            for (_cid, entry) in clients_lock.iter() {
                if entry.user_id != user_id && seen_users.insert(entry.user_id) {
                    let online_event = WsEvent {
                        event_type: "online".to_string(),
                        user_id: entry.user_id,
                        receiver_id: None,
                        group_id: None,
                        content: None,
                        data: None,
                    };
                    let _ = my_entry.sender.send(serde_json::to_string(&online_event).unwrap());
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
        let online_str = serde_json::to_string(&online_event).unwrap();
        for (cid, entry) in clients_lock.iter() {
            if *cid != conn_id && entry.user_id != user_id {
                let _ = entry.sender.send(online_str.clone());
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
                                                let ev_str = serde_json::to_string(&event).unwrap();
                                                for (_, entry) in clients_lock.iter() {
                                                    if entry.user_id == receiver_id {
                                                        let _ = entry.sender.send(ev_str.clone());
                                                    }
                                                }
                                            } else if let Some(group_id) = event.group_id {
                                                if let Ok(members) = GroupMembers::find()
                                                    .filter(group_members::Column::GroupId.eq(group_id))
                                                    .all(&db).await
                                                {
                                                    let ev_str = serde_json::to_string(&event).unwrap();
                                                    let member_ids: HashSet<i64> = members.iter().map(|m| m.user_id).collect();
                                                    for (_, entry) in clients_lock.iter() {
                                                        if member_ids.contains(&entry.user_id) && entry.user_id != user_id {
                                                            let _ = entry.sender.send(ev_str.clone());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        "typing" => {
                                            if let Some(receiver_id) = event.receiver_id {
                                                let clients_lock = clients.lock().await;
                                                let ev_str = serde_json::to_string(&event).unwrap();
                                                for (_, entry) in clients_lock.iter() {
                                                    if entry.user_id == receiver_id {
                                                        let _ = entry.sender.send(ev_str.clone());
                                                    }
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
                            tracing::info!("WS close from user {} conn {}", user_id, conn_id);
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        tracing::error!("WS error for user {} conn {}: {:?}", user_id, conn_id, e);
                        break;
                    }
                    None => {
                        tracing::info!("WS stream ended for user {} conn {}", user_id, conn_id);
                        break;
                    }
                }
            }
            Some(msg) = rx.recv() => {
                if let Err(e) = sink.send(WsMessage::text(msg)).await {
                    tracing::error!("Failed to send to user {} conn {}: {:?}", user_id, conn_id, e);
                    break;
                }
            }
            else => {
                tracing::info!("Both channels closed for user {} conn {}", user_id, conn_id);
                break;
            }
        }
    }

    // Unregister this connection and check if it's the last for this user
    let is_last_connection = {
        let mut clients_lock = clients.lock().await;
        clients_lock.remove(&conn_id);
        !clients_lock.values().any(|e| e.user_id == user_id)
    };

    // Broadcast offline status only for the last connection
    if is_last_connection {
        let clients_lock = clients.lock().await;
        let offline_event = WsEvent {
            event_type: "offline".to_string(),
            user_id,
            receiver_id: None,
            group_id: None,
            content: None,
            data: None,
        };
        let offline_str = serde_json::to_string(&offline_event).unwrap();
        for (_, entry) in clients_lock.iter() {
            if entry.user_id != user_id {
                let _ = entry.sender.send(offline_str.clone());
            }
        }
    }

    tracing::info!("WS cleanup done for user {} conn {}", user_id, conn_id);
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
