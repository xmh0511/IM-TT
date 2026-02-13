use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "messages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub sender_id: i64,
    pub receiver_id: Option<i64>,
    pub group_id: Option<i64>,
    pub content: String,
    pub message_type: String,
    pub created_at: DateTime,
    pub is_read: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::SenderId",
        to = "super::users::Column::Id"
    )]
    Sender,
    #[sea_orm(
        belongs_to = "super::users::Entity",
        from = "Column::ReceiverId",
        to = "super::users::Column::Id"
    )]
    Receiver,
    #[sea_orm(
        belongs_to = "super::groups::Entity",
        from = "Column::GroupId",
        to = "super::groups::Column::Id"
    )]
    Group,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sender.def()
    }
}

impl Related<super::groups::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Group.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
