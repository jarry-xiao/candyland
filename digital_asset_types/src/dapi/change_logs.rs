use sea_orm::DatabaseConnection;
use std::fmt::format;
use {
    crate::dao::asset,
    crate::dao::cl_items,
    sea_orm::{entity::*, query::*, DbErr},
};

pub async fn get_proof_for_asset(
    db: &DatabaseConnection,
    asset_id: Vec<u8>,
) -> Result<Vec<cl_items::Model>, DbErr> {
    cl_items::Entity::find()
        .join_rev(
            JoinType::InnerJoin,
            cl_items::Entity::belongs_to(asset::Entity)
                .from(cl_items::Column::Hash)
                .to(asset::Column::Leaf)
                .into(),
        )
        .order_by_asc(cl_items::Column::Level)
        .filter(asset::Column::Id.eq(asset_id))
        .all(db)
        .await
}
