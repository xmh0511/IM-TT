use salvo::prelude::*;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::models::{CreateGroupRequest, JoinGroupRequest};
use crate::entity::{groups, groups::Entity as Groups, group_members, group_members::Entity as GroupMembers};

#[handler]
pub async fn create_group(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let db = depot.get::<DatabaseConnection>("db").unwrap();
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

    let new_group = groups::ActiveModel {
        name: Set(group_data.name),
        description: Set(group_data.description),
        owner_id: Set(*user_id),
        ..Default::default()
    };

    match new_group.insert(db).await {
        Ok(group) => {
            // Add creator as owner member
            let new_member = group_members::ActiveModel {
                group_id: Set(group.id),
                user_id: Set(*user_id),
                role: Set("owner".to_string()),
                ..Default::default()
            };
            let _ = new_member.insert(db).await;

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
    let db = depot.get::<DatabaseConnection>("db").unwrap();
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
    let group_exists = Groups::find_by_id(join_data.group_id).one(db).await;

    if group_exists.is_err() || group_exists.unwrap().is_none() {
        res.status_code(StatusCode::NOT_FOUND);
        res.render(Json(serde_json::json!({
            "error": "Group not found"
        })));
        return;
    }

    let new_member = group_members::ActiveModel {
        group_id: Set(join_data.group_id),
        user_id: Set(*user_id),
        role: Set("member".to_string()),
        ..Default::default()
    };

    match new_member.insert(db).await {
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
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    let user_id = depot.get::<i64>("user_id").unwrap();

    // Get all groups where user is a member
    let members = GroupMembers::find()
        .filter(group_members::Column::UserId.eq(*user_id))
        .all(db)
        .await;

    match members {
        Ok(members) => {
            let mut groups = Vec::new();
            for member in members {
                if let Ok(Some(group)) = Groups::find_by_id(member.group_id).one(db).await {
                    groups.push(group);
                }
            }
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
    let db = depot.get::<DatabaseConnection>("db").unwrap();
    
    let group_id: i64 = match req.param::<String>("id") {
        Some(id) => id.parse().unwrap_or(0),
        None => 0,
    };

    let members = GroupMembers::find()
        .filter(group_members::Column::GroupId.eq(group_id))
        .all(db)
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
