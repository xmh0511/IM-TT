use salvo::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::models::{LoginRequest, RegisterRequest, AuthResponse};
use crate::entity::{users, users::Entity as Users};
use crate::utils::{hash_password, verify_password, create_token};

#[handler]
pub async fn register(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let db = depot.get::<DatabaseConnection>("db").unwrap();
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
    let existing_user = Users::find()
        .filter(
            users::Column::Email.eq(&register_data.email)
                .or(users::Column::Username.eq(&register_data.username))
        )
        .one(db)
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
    let new_user = users::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        username: Set(register_data.username.clone()),
        email: Set(register_data.email.clone()),
        password_hash: Set(password_hash),
        avatar: Set(None),
        status: Set("offline".to_string()),
        created_at: sea_orm::ActiveValue::NotSet,
        updated_at: sea_orm::ActiveValue::NotSet,
    };

    match new_user.insert(db).await {
        Ok(user) => {
            // Generate JWT token
            let token = match create_token(user.id, jwt_secret) {
                Ok(t) => t,
                Err(_) => {
                    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                    res.render(Json(serde_json::json!({
                        "error": "Failed to generate authentication token"
                    })));
                    return;
                }
            };
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
    let db = depot.get::<DatabaseConnection>("db").unwrap();
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
    let user = Users::find()
        .filter(users::Column::Email.eq(&login_data.email))
        .one(db)
        .await;

    match user {
        Ok(Some(user)) => {
            // Verify password
            match verify_password(&login_data.password, &user.password_hash) {
                Ok(true) => {
                    // Generate JWT token
                    let token = match create_token(user.id, jwt_secret) {
                        Ok(t) => t,
                        Err(_) => {
                            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                            res.render(Json(serde_json::json!({
                                "error": "Failed to generate authentication token"
                            })));
                            return;
                        }
                    };

                    // Update user status to online
                    let mut user_active: users::ActiveModel = user.clone().into();
                    user_active.status = Set("online".to_string());
                    let _ = user_active.update(db).await;

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
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let user = Users::find_by_id(*user_id).one(db).await;

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
