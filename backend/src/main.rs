mod models;
mod handlers;
mod db;
mod utils;
mod websocket;
mod entity;

use salvo::prelude::*;
use salvo::cors::{Cors, CorsHandler};
use salvo::http::Method;
use tracing_subscriber;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use once_cell::sync::OnceCell;

// Global application state
pub static APP_STATE: OnceCell<AppState> = OnceCell::new();

// Application shared state
#[derive(Clone,Debug)]
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

// Middleware to inject app state into depot
#[handler]
async fn inject_app_state(depot: &mut Depot, req: &mut Request, res: &mut Response, ctrl: &mut FlowCtrl) {
    let app_state = AppState::global();
    depot.insert("db", app_state.db.as_ref().clone());
    depot.insert("jwt_secret", app_state.jwt_secret.as_ref().clone());
    depot.inject(app_state.clients.clone());
    ctrl.call_next(req, depot, res).await;
}

// Middleware to verify JWT token
#[handler]
async fn auth_middleware(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if let Some(token) = token {
        let app_state = AppState::global();
        match utils::verify_token(token, &app_state.jwt_secret) {
            Ok(claims) => {
                depot.insert("user_id", claims.sub);
                ctrl.call_next(req, depot, res).await;
                return;
            }
            Err(_) => {}
        }
    }

    res.status_code(StatusCode::UNAUTHORIZED);
    res.render(Json(serde_json::json!({
        "error": "Invalid or expired token"
    })));
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL")
        .expect("REDIS_URL must be set");
    let jwt_secret = env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    let server_host = env::var("SERVER_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    let server_port = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string());

    // Initialize database
    let db = db::create_connection(&database_url)
        .await
        .expect("Failed to create database connection");

    db::run_migrations(&db)
        .await
        .expect("Failed to run migrations");

    // Initialize Redis
    let _redis_client = db::create_redis_client(&redis_url)
        .await
        .expect("Failed to create Redis client");

    // Create WebSocket clients map
    let clients = websocket::create_clients();

    // Create and initialize global app state
    let app_state = AppState {
        db: Arc::new(db),
        jwt_secret: Arc::new(jwt_secret),
        clients: clients.clone(),
    };

    APP_STATE.set(app_state).expect("Failed to set APP_STATE");

    // Configure CORS
    let cors_handler: CorsHandler = Cors::new()
        .allow_origin("*")
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .into_handler();

    let router = Router::new()
        .push(
            Router::with_path("/api")
                .hoop(inject_app_state)
                .push(
                    Router::with_path("/auth")
                        .push(Router::with_path("/register").post(handlers::register))
                        .push(Router::with_path("/login").post(handlers::login))
                )
                .push(
                    Router::with_path("/user")
                        .hoop(auth_middleware)
                        .push(Router::with_path("/me").get(handlers::get_current_user))
                )
                .push(
                    Router::with_path("/messages")
                        .hoop(auth_middleware)
                        .push(Router::with_path("/send").post(handlers::send_message))
                        .push(Router::with_path("/list").get(handlers::get_messages))
                        .push(Router::with_path("/<id>/read").put(handlers::mark_as_read))
                )
                .push(
                    Router::with_path("/groups")
                        .hoop(auth_middleware)
                        .push(Router::with_path("/create").post(handlers::create_group))
                        .push(Router::with_path("/join").post(handlers::join_group))
                        .push(Router::with_path("/list").get(handlers::get_user_groups))
                        .push(Router::with_path("/<id>/members").get(handlers::get_group_members))
                )
                .push(
                    Router::with_path("/ws")
                        .hoop(auth_middleware)
                        .goal(websocket::websocket_handler)
                )
        );

    let acceptor = TcpListener::new(format!("{}:{}", server_host, server_port))
        .bind()
        .await;

    tracing::info!("Server running on http://{}:{}", server_host, server_port);
    let service = Service::new(router).hoop(cors_handler);
    Server::new(acceptor).serve(service).await;
}
