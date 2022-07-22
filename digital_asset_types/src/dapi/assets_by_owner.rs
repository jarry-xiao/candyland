use crate::dao::prelude::{Asset, AssetData};
use crate::dao::{asset, asset_authority, asset_creators, asset_data, asset_grouping, cl_items};
use crate::dapi::asset::{get_content, to_authority, to_creators, to_grouping};
use crate::dapi::utils::asset_grouping;
use crate::rpc::filter::AssetSorting;
use crate::rpc::response::AssetList;
use crate::rpc::{
    Asset as RpcAsset, Authority, Compression, Content, Creator, File, Group, Interface, Links,
    Ownership, Royalty, Scope,
};
use futures::future::join_all;
use jsonpath_lib::JsonPathError;
use mime_guess::Mime;
use sea_orm::DatabaseConnection;
use sea_orm::{entity::*, query::*, DbErr};

pub async fn get_assets_by_owner(
    db: &DatabaseConnection,
    owner_address: Vec<u8>,
    sort_by: AssetSorting,
    limit: u32,
    page: u32,
    before: String,
    after: String,
) -> Result<AssetList, DbErr> {
    let paginator = Asset::find()
        .filter(asset::Column::Owner.eq(owner_address.clone()))
        .find_also_related(AssetData)
        .order_by_asc(asset::Column::CreatedAt)
        .paginate(db, limit.try_into().unwrap());

    let num_pages = paginator.num_pages().await.unwrap();

    let assets = paginator.fetch_page((page - 1).try_into().unwrap()).await?;

    let built_assets = join_all(assets.into_iter().map(|(asset, data)| async move {
        let interface = match asset.specification_version {
            1 => Interface::NftOneZero,
            _ => Interface::Nft,
        };

        let content = if let Some(data) = data {
            let res = get_content(&asset, &data).unwrap();
            Some(res)
        } else {
            None
        };

        let authorities = asset_authority::Entity::find()
            .filter(asset_authority::Column::AssetId.eq(asset.id.clone()))
            .all(db);

        let creators = asset_creators::Entity::find()
            .filter(asset_creators::Column::AssetId.eq(asset.id.clone()))
            .all(db);

        let grouping = asset_grouping::Entity::find()
            .filter(asset_grouping::Column::AssetId.eq(asset.id.clone()))
            .all(db);

        //             let rpc_authorities = to_authority(authorities);
        //             let rpc_creators = to_creators(creators);
        //             let rpc_groups = to_grouping(grouping);

        grouping
    }))
    .await;

    // let handles: Vec<RpcAsset> = join_all(
    //     asset_list
    //         .into_iter()
    //         .map(|(asset, data)| async move {
    //             let interface = match asset.specification_version {
    //                 1 => Interface::NftOneZero,
    //                 _ => Interface::Nft,
    //             };

    //             let content = if let Some(data) = data {
    //                 let res = get_content(&asset, &data).unwrap();
    //                 Some(res)
    //             } else {
    //                 None
    //             };

    //             let grouping = asset_grouping(asset, db)?;
    //             let rpc_authorities = to_authority(authorities);
    //             let rpc_creators = to_creators(creators);
    //             let rpc_groups = to_grouping(grouping);

    //             RpcAsset {
    //                 interface,
    //                 id: bs58::encode(asset.id).into_string(),
    //                 content,
    //                 authorities: Some(rpc_authorities),
    //                 compression: Some(Compression {
    //                     eligible: asset.compressible,
    //                     compressed: asset.compressed,
    //                 }),
    //                 grouping: Some(rpc_groups),
    //                 royalty: Some(Royalty {
    //                     royalty_model: asset.royalty_target_type.into(),
    //                     target: asset.royalty_target.map(|s| bs58::encode(s).into_string()),
    //                     percent: (asset.royalty_amount as f64) * 0.0001,
    //                     locked: false,
    //                 }),
    //                 creators: Some(rpc_creators),
    //                 ownership: Ownership {
    //                     frozen: asset.frozen,
    //                     delegated: asset.delegate.is_some(),
    //                     delegate: asset.delegate.map(|s| bs58::encode(s).into_string()),
    //                     ownership_model: asset.owner_type.into(),
    //                     owner: bs58::encode(asset.owner).into_string(),
    //                 },
    //             }
    //         })
    //         .collect::<Vec<RpcAsset>>(),
    // )
    // .await;

    Ok(AssetList {
        total: todo!(),
        limit,
        page: todo!(),
        before: todo!(),
        after: todo!(),
        items: todo!(),
    })
}
