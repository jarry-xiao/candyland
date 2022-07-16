use crate::dao::prelude::{Asset, AssetData};
use crate::dao::{asset, asset_authority, asset_creators, asset_data, asset_grouping, cl_items};
use crate::rpc::filter::AssetSorting;
use crate::rpc::response::AssetList;
use crate::rpc::{
    Asset as RpcAsset, Authority, Compression, Content, Creator, File, Group, Interface, Links,
    Ownership, Royalty, Scope,
};
use jsonpath_lib::JsonPathError;
use mime_guess::Mime;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DbErr};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use url::Url;

pub async fn get_assets_by_owner(
    db: &DatabaseConnection,
    owner_address: Vec<u8>,
    sort_by: AssetSorting,
    limit: u32,
    page: u32,
    before: String,
    after: String,
) -> Result<AssetList, DbErr> {
  

    let asset_list = cl_items::Entity::find().all(db).await?;
    Ok(())
}
