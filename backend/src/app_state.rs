use crate::websocket;
use once_cell::sync::OnceCell;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// Global application state
pub static APP_STATE: OnceCell<AppState> = OnceCell::new();

// Application shared state
#[derive(Clone, Debug)]
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
