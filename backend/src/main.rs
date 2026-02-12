mod models;
mod handlers;
mod db;
mod utils;
mod websocket;

use salvo::prelude::*;
use salvo::cors::{Cors, CorsHandler};
use tracing_subscriber;
use dotenv::dotenv;
use std::env;

// Middleware to verify JWT token
#[handler]
async fn auth_middleware(req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    if let Some(token) = token {
        let jwt_secret = depot.obtain::<String>().unwrap();
        match utils::verify_token(token, jwt_secret) {
            Ok(claims) => {
                depot.insert(claims.sub);
                ctrl.call_next(req, depot, res).await;
            }
            Err(_) => {
                res.status_code(StatusCode::UNAUTHORIZED);
                res.render(Json(serde_json::json!({
                    "error": "Invalid or expired token"
                })));
            }
        }
    } else {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(serde_json::json!({
            "error": "No token provided"
        })));
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

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
    let pool = db::create_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    // Initialize Redis
    let redis_client = db::create_redis_client(&redis_url)
        .await
        .expect("Failed to create Redis client");

    // Create WebSocket clients map
    let clients = websocket::create_clients();

    // Configure CORS
    let cors_handler: CorsHandler = Cors::new()
        .allow_origin("*")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .into_handler();

    // Build router
    let router = Router::new()
        .hoop(
            affix::inject(pool.clone())
                .inject(jwt_secret.clone())
                .inject(clients.clone())
        )
        .push(
            Router::with_path("/api")
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
        )
        .hoop(cors_handler);

    let acceptor = TcpListener::new(format!("{}:{}", server_host, server_port))
        .bind()
        .await;

    tracing::info!("Server running on http://{}:{}", server_host, server_port);
    Server::new(acceptor).serve(router).await;
}
