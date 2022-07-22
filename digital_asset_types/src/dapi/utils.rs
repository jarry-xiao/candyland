use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use crate::dao::{asset, asset_grouping};

pub fn asset_grouping(
    asset: asset::Model,
    db: &DatabaseConnection,
) -> Result<Vec<asset_grouping::Model>, DbErr> {
    asset_grouping::Entity::find()
        .filter(asset_grouping::Column::AssetId.eq(asset.id.clone()))
        .all(db)
}
