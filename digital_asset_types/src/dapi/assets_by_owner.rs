use crate::dao::prelude::{Asset, AssetData};
use crate::dao::{asset, asset_authority, asset_creators, asset_grouping};
use crate::dapi::asset::{get_content, to_authority, to_creators, to_grouping};
use crate::rpc::filter::AssetSorting;
use crate::rpc::response::AssetList;
use crate::rpc::{Asset as RpcAsset, Compression, Interface, Ownership, Royalty};
use futures::FutureExt;
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
    let sort_column = match sort_by {
        AssetSorting::Created => asset::Column::CreatedAt,
        AssetSorting::Updated => todo!(),
        AssetSorting::RecentAction => todo!(),
    };

    let assets = if page > 0 {
        let paginator = Asset::find()
            .filter(asset::Column::Owner.eq(owner_address.clone()))
            .find_also_related(AssetData)
            .order_by_asc(sort_column)
            .paginate(db, limit.try_into().unwrap());

        paginator.fetch_page((page - 1).try_into().unwrap()).await?
    }
    // else if !before.is_empty() {
    //     let mut cursor = Asset::find()
    //         .filter(asset::Column::Owner.eq(owner_address.clone()))
    //         .cursor_by(asset::Column::Id);

    //     let assets = cursor
    //         .before(before)
    //         .first(limit.into())
    //         .order_by_asc(sort_column.clone())
    //         .all(db)
    //         .await?
    //         .into_iter()
    //         .map(|x| async move {
    //             let asset_data = x.find_related(AssetData).one(db).await.unwrap();

    //             (x, asset_data)
    //         });

    //     let awaited = futures::future::join_all(assets).await;
    //     awaited
    // }
    else {
        // let rows = asset::Entity::find()
        //     .filter(asset::Column::Owner.eq(owner_address.clone()))
        //     .cursor_by(asset::Column::Id)
        //     .after(after)
        //     .first(limit.into())
        //     .all(db)
        //     .await?;

        let rows = asset::Entity::find()
            .filter(asset::Column::Owner.eq(owner_address.clone()))
            .cursor_by(asset::Column::Id)
            .after(after)
            .first(limit.into())
            .all(db)
            .await?
            .into_iter()
            .map(|x| async move {
                let asset_data = x.find_related(AssetData).one(db).await.unwrap();

                (x, asset_data)
            });

        let assets = futures::future::join_all(rows).await;
        assets
    };

    let filter_assets: Result<Vec<_>, _> = assets
        .into_iter()
        .map(|(asset, asset_data)| match asset_data {
            Some(asset_data) => Ok((asset, asset_data)),
            _ => Err(DbErr::RecordNotFound("Asset Not Found".to_string())),
        })
        .collect();

    let build_asset_list = filter_assets?
        .into_iter()
        .map(|(asset, asset_data)| async move {
            let interface = match asset.specification_version {
                1 => Interface::NftOneZero,
                _ => Interface::Nft,
            };

            let content = get_content(&asset, &asset_data).unwrap();

            let authorities = asset_authority::Entity::find()
                .filter(asset_authority::Column::AssetId.eq(asset.id.clone()))
                .all(db)
                .await
                .unwrap();

            let creators = asset_creators::Entity::find()
                .filter(asset_creators::Column::AssetId.eq(asset.id.clone()))
                .all(db)
                .await
                .unwrap();

            let grouping = asset_grouping::Entity::find()
                .filter(asset_grouping::Column::AssetId.eq(asset.id.clone()))
                .all(db)
                .await
                .unwrap();

            let rpc_authorities = to_authority(authorities);
            let rpc_creators = to_creators(creators);
            let rpc_groups = to_grouping(grouping);

            RpcAsset {
                interface,
                id: bs58::encode(asset.id).into_string(),
                content: Some(content),
                authorities: Some(rpc_authorities),
                compression: Some(Compression {
                    eligible: asset.compressible,
                    compressed: asset.compressed,
                }),
                grouping: Some(rpc_groups),
                royalty: Some(Royalty {
                    royalty_model: asset.royalty_target_type.into(),
                    target: asset.royalty_target.map(|s| bs58::encode(s).into_string()),
                    percent: (asset.royalty_amount as f64) * 0.0001,
                    locked: false,
                }),
                creators: Some(rpc_creators),
                ownership: Ownership {
                    frozen: asset.frozen,
                    delegated: asset.delegate.is_some(),
                    delegate: asset.delegate.map(|s| bs58::encode(s).into_string()),
                    ownership_model: asset.owner_type.into(),
                    owner: bs58::encode(asset.owner).into_string(),
                },
            }
        });

    let built_assets = futures::future::join_all(build_asset_list).await;

    let total = built_assets.len() as u32;

    Ok(AssetList {
        total,
        limit,
        page: Some(page),
        before: None,
        after: None,
        items: built_assets,
    })
}
