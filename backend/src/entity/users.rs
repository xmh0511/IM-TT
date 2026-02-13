use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub username: String,
    #[sea_orm(unique)]
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub avatar: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "Timestamp")]
    pub created_at: Option<DateTime>,
    #[sea_orm(column_type = "Timestamp")]
    pub updated_at: Option<DateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::messages::Entity")]
    Messages,
    #[sea_orm(has_many = "super::groups::Entity")]
    Groups,
    #[sea_orm(has_many = "super::group_members::Entity")]
    GroupMembers,
}

impl Related<super::messages::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

impl Related<super::groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Groups.def()
    }
}

impl Related<super::group_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GroupMembers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
