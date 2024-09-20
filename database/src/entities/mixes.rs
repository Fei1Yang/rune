//! `SeaORM` Entity. Generated by sea-orm-codegen 0.12.15

use async_graphql::SimpleObject;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, SimpleObject)]
#[sea_orm(table_name = "mixes")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub name: String,
    pub group: String,
    pub scriptlet_mode: bool,
    pub mode: i32,
    pub locked: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::mix_queries::Entity")]
    MixQueries,
}

impl Related<super::mix_queries::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::MixQueries.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelatedEntity)]
pub enum RelatedEntity {
    #[sea_orm(entity = "super::mix_queries::Entity")]
    MixQueries,
}
