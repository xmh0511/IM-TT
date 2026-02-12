use salvo::prelude::*;
use sqlx::MySqlPool;
use crate::models::{LoginRequest, RegisterRequest, AuthResponse, User};
use crate::utils::{hash_password, verify_password, create_token};

#[handler]
pub async fn register(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let jwt_secret = depot.get::<String>("jwt_secret").unwrap();
    
    let register_data = match req.parse_json::<RegisterRequest>().await {
        Ok(data) => data,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "Invalid request data"
            })));
            return;
        }
    };

    // Check if user already exists
    let existing_user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = ? OR username = ?"
    )
    .bind(&register_data.email)
    .bind(&register_data.username)
    .fetch_optional(pool)
    .await;

    if let Ok(Some(_)) = existing_user {
        res.status_code(StatusCode::CONFLICT);
        res.render(Json(serde_json::json!({
            "error": "User already exists"
        })));
        return;
    }

    // Hash password
    let password_hash = match hash_password(&register_data.password) {
        Ok(hash) => hash,
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to hash password"
            })));
            return;
        }
    };

    // Insert user
    let result = sqlx::query(
        "INSERT INTO users (username, email, password_hash) VALUES (?, ?, ?)"
    )
    .bind(&register_data.username)
    .bind(&register_data.email)
    .bind(&password_hash)
    .execute(pool)
    .await;

    match result {
        Ok(query_result) => {
            let user_id = query_result.last_insert_id() as i64;
            
            // Fetch the created user
            let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap();

            // Generate JWT token
            let token = create_token(user_id, jwt_secret).unwrap();

            res.render(Json(AuthResponse { token, user }));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to create user"
            })));
        }
    }
}

#[handler]
pub async fn login(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let jwt_secret = depot.get::<String>("jwt_secret").unwrap();

    let login_data = match req.parse_json::<LoginRequest>().await {
        Ok(data) => data,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "Invalid request data"
            })));
            return;
        }
    };

    // Find user by email
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&login_data.email)
        .fetch_optional(pool)
        .await;

    match user {
        Ok(Some(user)) => {
            // Verify password
            match verify_password(&login_data.password, &user.password_hash) {
                Ok(true) => {
                    // Generate JWT token
                    let token = create_token(user.id, jwt_secret).unwrap();

                    // Update user status to online
                    let _ = sqlx::query("UPDATE users SET status = 'online' WHERE id = ?")
                        .bind(user.id)
                        .execute(pool)
                        .await;

                    res.render(Json(AuthResponse { token, user }));
                }
                _ => {
                    res.status_code(StatusCode::UNAUTHORIZED);
                    res.render(Json(serde_json::json!({
                        "error": "Invalid credentials"
                    })));
                }
            }
        }
        _ => {
            res.status_code(StatusCode::UNAUTHORIZED);
            res.render(Json(serde_json::json!({
                "error": "Invalid credentials"
            })));
        }
    }
}

#[handler]
pub async fn get_current_user(res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(pool)
        .await;

    match user {
        Ok(Some(user)) => {
            res.render(Json(user));
        }
        _ => {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Json(serde_json::json!({
                "error": "User not found"
            })));
        }
    }
}
