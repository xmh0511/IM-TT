use salvo::prelude::*;
use sqlx::MySqlPool;
use crate::models::{CreateGroupRequest, JoinGroupRequest, Group, GroupMember};

#[handler]
pub async fn create_group(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let group_data = match req.parse_json::<CreateGroupRequest>().await {
        Ok(data) => data,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "Invalid request data"
            })));
            return;
        }
    };

    let result = sqlx::query(
        "INSERT INTO groups_table (name, description, owner_id) VALUES (?, ?, ?)"
    )
    .bind(&group_data.name)
    .bind(&group_data.description)
    .bind(user_id)
    .execute(pool)
    .await;

    match result {
        Ok(query_result) => {
            let group_id = query_result.last_insert_id() as i64;
            
            // Add creator as owner member
            let _ = sqlx::query(
                "INSERT INTO group_members (group_id, user_id, role) VALUES (?, ?, 'owner')"
            )
            .bind(group_id)
            .bind(user_id)
            .execute(pool)
            .await;

            let group = sqlx::query_as::<_, Group>("SELECT * FROM groups_table WHERE id = ?")
                .bind(group_id)
                .fetch_one(pool)
                .await
                .unwrap();

            res.render(Json(group));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to create group"
            })));
        }
    }
}

#[handler]
pub async fn join_group(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let join_data = match req.parse_json::<JoinGroupRequest>().await {
        Ok(data) => data,
        Err(_) => {
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(serde_json::json!({
                "error": "Invalid request data"
            })));
            return;
        }
    };

    // Check if group exists
    let group_exists = sqlx::query_as::<_, Group>("SELECT * FROM groups_table WHERE id = ?")
        .bind(join_data.group_id)
        .fetch_optional(pool)
        .await;

    if group_exists.is_err() || group_exists.unwrap().is_none() {
        res.status_code(StatusCode::NOT_FOUND);
        res.render(Json(serde_json::json!({
            "error": "Group not found"
        })));
        return;
    }

    let result = sqlx::query(
        "INSERT INTO group_members (group_id, user_id, role) VALUES (?, ?, 'member')"
    )
    .bind(join_data.group_id)
    .bind(user_id)
    .execute(pool)
    .await;

    match result {
        Ok(_) => {
            res.render(Json(serde_json::json!({
                "success": true,
                "message": "Joined group successfully"
            })));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to join group"
            })));
        }
    }
}

#[handler]
pub async fn get_user_groups(res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    let groups = sqlx::query_as::<_, Group>(
        "SELECT g.* FROM groups_table g 
         INNER JOIN group_members gm ON g.id = gm.group_id 
         WHERE gm.user_id = ?"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await;

    match groups {
        Ok(groups) => {
            res.render(Json(groups));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to fetch groups"
            })));
        }
    }
}

#[handler]
pub async fn get_group_members(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let pool = depot.obtain::<MySqlPool>().unwrap();
    
    let group_id: i64 = match req.param("id") {
        Some(id) => id.parse().unwrap_or(0),
        None => 0,
    };

    let members = sqlx::query_as::<_, GroupMember>(
        "SELECT * FROM group_members WHERE group_id = ?"
    )
    .bind(group_id)
    .fetch_all(pool)
    .await;

    match members {
        Ok(members) => {
            res.render(Json(members));
        }
        Err(_) => {
            res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            res.render(Json(serde_json::json!({
                "error": "Failed to fetch group members"
            })));
        }
    }
}
