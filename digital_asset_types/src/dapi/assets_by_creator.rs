use crate::dao::prelude::{Asset, AssetCreators, AssetData};
use crate::dao::{asset, asset_authority, asset_creators, asset_grouping};
use crate::dapi::asset::{get_content, to_authority, to_creators, to_grouping};
use crate::rpc::filter::AssetSorting;
use crate::rpc::response::AssetList;
use crate::rpc::{Asset as RpcAsset, Compression, Interface, Ownership, Royalty};
use sea_orm::{entity::*, query::*, DbBackend, DbErr};
use sea_orm::{DatabaseBackend, DatabaseConnection};

pub async fn get_assets_by_creator(
    db: &DatabaseConnection,
    creator_expression: Vec<Vec<u8>>,
    sort_by: AssetSorting,
    limit: u32,
    page: u32,
    before: Vec<u8>,
    after: Vec<u8>,
) -> Result<AssetList, DbErr> {
    let sort_column = match sort_by {
        AssetSorting::Created => asset::Column::CreatedAt,
        AssetSorting::Updated => todo!(),
        AssetSorting::RecentAction => todo!(),
    };

    let assets = if page > 0 {
        //     let result = asset_creators::Entity::find().from_raw_sql(Statement::from_sql_and_values(
        //     DbBackend::Postgres,
        //     r#"SELECT "asset_creators"."id" AS "A_id", "asset_creators"."asset_id" AS "A_asset_id", "asset_creators"."creator" AS "A_creator", "asset_creators"."share" AS "A_share", "asset_creators"."verified" AS "A_verified", "asset"."id" AS "B_id", "asset"."specification_version" AS "B_specification_version", "asset"."owner" AS "B_owner", CAST("asset"."owner_type" AS text) AS "B_owner_type", "asset"."delegate" AS "B_delegate", "asset"."frozen" AS "B_frozen", "asset"."supply" AS "B_supply", "asset"."supply_mint" AS "B_supply_mint", "asset"."compressed" AS "B_compressed", "asset"."compressible" AS "B_compressible", "asset"."tree_id" AS "B_tree_id", "asset"."leaf" AS "B_leaf", "asset"."nonce" AS "B_nonce", CAST("asset"."royalty_target_type" AS text) AS "B_royalty_target_type", "asset"."royalty_target" AS "B_royalty_target", "asset"."royalty_amount" AS "B_royalty_amount", "asset"."chain_data_id" AS "B_chain_data_id", "asset"."created_at" AS "B_created_at", "asset"."burnt" AS "B_burnt" FROM "asset_creators" LEFT JOIN "asset" ON "asset_creators"."asset_id" = "asset"."id" "#,
        //     vec![],
        // )).all(db).await?;

        let paginator = AssetCreators::find()
            .filter(
                Condition::any()
                    .add(asset_creators::Column::Creator.eq(creator_expression[0].clone())), // .add(asset_creators::Column::Creator.eq(creator_expression[1].clone())),
            )
            .find_also_related(Asset)
            .order_by_asc(sort_column)
            .paginate(db, limit.try_into().unwrap());

        let rows = paginator.fetch_page((page - 1).try_into().unwrap()).await?;

        let get_asset_data = rows.into_iter().map(|(_creator, asset)| async move {
            let asset_data = asset
                .as_ref()
                .unwrap()
                .find_related(AssetData)
                .one(db)
                .await
                .unwrap();

            (asset, asset_data)
        });

        let assets = futures::future::join_all(get_asset_data).await;
        assets
    } else if !before.is_empty() {
        let rows = asset_creators::Entity::find()
            .order_by_asc(sort_column)
            .filter(
                Condition::all()
                    .add(asset_creators::Column::Creator.eq(creator_expression[0].clone())), // .add(asset_creators::Column::Creator.eq(creator_expression[1].clone())),
            )
            .cursor_by(asset_creators::Column::AssetId)
            .before(before)
            .first(limit.into())
            .all(db)
            .await?
            .into_iter()
            .map(|x| async move {
                let asset = x.find_related(Asset).one(db).await.unwrap();
                let asset_data = asset
                    .as_ref()
                    .unwrap()
                    .find_related(AssetData)
                    .one(db)
                    .await
                    .unwrap();

                (asset, asset_data)
            });

        let assets = futures::future::join_all(rows).await;
        assets
    } else {
        let rows = asset_creators::Entity::find()
            .order_by_asc(sort_column)
            .filter(
                Condition::all()
                    .add(asset_creators::Column::Creator.eq(creator_expression[0].clone())), // .add(asset_creators::Column::Creator.eq(creator_expression[1].clone())),
            )
            .cursor_by(asset_creators::Column::AssetId)
            .after(after)
            .first(limit.into())
            .all(db)
            .await?
            .into_iter()
            .map(|x| async move {
                let asset = x.find_related(Asset).one(db).await.unwrap();
                let asset_data = asset
                    .as_ref()
                    .unwrap()
                    .find_related(AssetData)
                    .one(db)
                    .await
                    .unwrap();

                (asset, asset_data)
            });

        let assets = futures::future::join_all(rows).await;
        assets
    };

    let filter_assets: Result<Vec<_>, _> = assets
        .into_iter()
        .map(|(asset, asset_data)| match (asset, asset_data) {
            (Some(asset), Some(asset_data)) => Ok((asset, asset_data)),
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

            println!("asset {:?}", asset);
            println!("asset {:?}", asset);
            println!("asset {:?}", asset);

            println!("asset data {:?}", asset_data);
            println!("asset data {:?}", asset_data);
            println!("asset data {:?}", asset_data);
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
