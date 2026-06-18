use salvo::prelude::*;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
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

pub type Clients = Arc<Mutex<HashMap<i64, broadcast::Sender<String>>>>;

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

    WebSocketUpgrade::new()
        .upgrade(req, res, move |ws| handle_socket(ws, user_id, clients, db))
        .await
}

async fn handle_socket(ws: WebSocket, user_id: i64, clients: Clients, db: DatabaseConnection) {
    let (tx, _rx) = broadcast::channel::<String>(100);

    {
        let mut clients_lock = clients.lock().await;
        clients_lock.insert(user_id, tx.clone());
    }

    // Broadcast online status
    {
        let online_event = WsEvent {
            event_type: "online".to_string(),
            user_id,
            receiver_id: None,
            group_id: None,
            content: None,
            data: None,
        };
        let clients_lock = clients.lock().await;
        for (uid, sender) in clients_lock.iter() {
            if *uid != user_id {
                let _ = sender.send(serde_json::to_string(&online_event).unwrap());
            }
        }
    }

    let (mut ws_tx, mut ws_rx) = ws.split();

    let mut rx_clone = tx.subscribe();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx_clone.recv().await {
            if ws_tx.send(WsMessage::text(msg)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Ok(text) = msg.as_str() {
            if let Ok(event) = serde_json::from_str::<WsEvent>(text) {
                match event.event_type.as_str() {
                    "message" => {
                        if let Some(content) = &event.content {
                            let _ = save_message(
                                &db,
                                user_id,
                                event.receiver_id,
                                event.group_id,
                                content,
                                "text",
                            ).await;
                        }

                        let clients_lock = clients.lock().await;

                        if let Some(receiver_id) = event.receiver_id {
                            if let Some(sender) = clients_lock.get(&receiver_id) {
                                let _ = sender.send(serde_json::to_string(&event).unwrap());
                            }
                        } else if let Some(group_id) = event.group_id {
                            let members = GroupMembers::find()
                                .filter(group_members::Column::GroupId.eq(group_id))
                                .all(&db)
                                .await;

                            if let Ok(members) = members {
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

    send_task.abort();
    let mut clients_lock = clients.lock().await;
    clients_lock.remove(&user_id);
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
