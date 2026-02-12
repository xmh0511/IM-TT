use std::sync::Arc;
use sea_orm::DatabaseConnection;
use once_cell::sync::OnceCell;
use crate::websocket;

// Global application state
pub static APP_STATE: OnceCell<AppState> = OnceCell::new();

// Application shared state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub jwt_secret: Arc<String>,
    pub clients: websocket::Clients,
}

impl AppState {
    pub fn global() -> &'static AppState {
        APP_STATE.get().expect("AppState not initialized")
    }
}
