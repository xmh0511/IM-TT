use salvo::prelude::*;
use salvo::websocket::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use serde::{Deserialize, Serialize};
use futures_util::{StreamExt, SinkExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEvent {
    pub event_type: String, // message, typing, online, offline
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
    let user_id = depot.get::<i64>("user_id").cloned();
    let clients = depot.get::<Clients>("clients").cloned();

    if user_id.is_none() || clients.is_none() {
        return Err(salvo::http::StatusError::unauthorized());
    }

    let user_id = user_id.unwrap();
    let clients = clients.unwrap();

    WebSocketUpgrade::new()
        .upgrade(req, res, move |ws| handle_socket(ws, user_id, clients))
        .await
}

async fn handle_socket(ws: WebSocket, user_id: i64, clients: Clients) {
    let (tx, _rx) = broadcast::channel::<String>(100);
    
    // Register client
    {
        let mut clients_lock = clients.lock().await;
        clients_lock.insert(user_id, tx.clone());
    }

    let (mut ws_tx, mut ws_rx) = ws.split();

    // Spawn a task to send messages from broadcast to WebSocket
    let mut rx_clone = tx.subscribe();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx_clone.recv().await {
            if ws_tx.send(WsMessage::text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = ws_rx.next().await {
        if let Ok(text) = msg.to_str() {
            if let Ok(event) = serde_json::from_str::<WsEvent>(text) {
                // Broadcast to relevant receivers
                let clients_lock = clients.lock().await;
                
                match event.event_type.as_str() {
                    "message" => {
                        if let Some(receiver_id) = event.receiver_id {
                            // Personal message
                            if let Some(sender) = clients_lock.get(&receiver_id) {
                                let _ = sender.send(serde_json::to_string(&event).unwrap());
                            }
                        } else if let Some(_group_id) = event.group_id {
                            // Group message - broadcast to all group members
                            // In production, you'd query the database for group members
                            // For now, we'll just echo back
                            for (_, sender) in clients_lock.iter() {
                                let _ = sender.send(serde_json::to_string(&event).unwrap());
                            }
                        }
                    }
                    "typing" => {
                        if let Some(receiver_id) = event.receiver_id {
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

    // Cleanup on disconnect
    send_task.abort();
    let mut clients_lock = clients.lock().await;
    clients_lock.remove(&user_id);
}
