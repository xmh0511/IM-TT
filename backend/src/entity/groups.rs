use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "groups_table")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub avatar: Option<String>,
    pub owner_id: i64,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::OwnerId",
        to = "super::users::Column::Id"
    )]
    Owner,
    #[sea_orm(has_many = "super::messages::Entity")]
    Messages,
    #[sea_orm(has_many = "super::group_members::Entity")]
    GroupMembers,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl Related<super::messages::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Messages.def()
    }
}

impl Related<super::group_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GroupMembers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
