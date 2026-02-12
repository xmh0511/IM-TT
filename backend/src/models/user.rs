use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: crate::entity::users::Model,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // user id
    pub exp: usize,
}

