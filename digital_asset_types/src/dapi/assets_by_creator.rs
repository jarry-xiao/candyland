use crate::dao::prelude::AssetData;
use crate::dao::{asset, asset_authority, asset_creators, asset_grouping};
use crate::dapi::asset::{get_content, to_authority, to_creators, to_grouping};
use crate::dapi::assets_by_creator::asset::Relation::AssetCreators;
use crate::dapi::assets_by_creator::asset_creators::Relation::Asset;
use crate::rpc::filter::AssetSorting;
use crate::rpc::response::AssetList;
use crate::rpc::{Asset as RpcAsset, Compression, Interface, Ownership, Royalty};
use sea_orm::{entity::*, query::*, DbErr, FromQueryResult};
use sea_orm::{DatabaseConnection, DbBackend};

#[derive(FromQueryResult)]
struct CakeAndFillingCount {
    id: i32,
    name: String,
    count: i32,
}

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

    // TODO: throw error if cursor and page pagination are included
    // TODO: returning proper
    let assets = if page > 0 {
        let mut cake_pages = asset_creators::Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::Postgres,
                r#"SELECT "ac"."asset_id", "ac"."creator" FROM "asset_creators" AS "ac"
                    LEFT OUTER JOIN "asset" AS "a" ON "a"."id" = "ac"."asset_id" 
                     WHERE "ac"."creator" = $1"#,
                vec![1.into()],
            ))
            .into_model::<CakeAndFillingCount>()
            .paginate(db, 50);

        let paginator = asset::Entity::find()
            .join(
                JoinType::LeftJoin,
                asset::Entity::has_many(asset_creators::Entity).into(),
            )
            .filter(
                sea_orm::Condition::any()
                    .add(asset_creators::Column::Creator.eq(creator_expression[0].clone())),
            )
            .find_also_related(AssetData)
            .order_by_asc(sort_column)
            .paginate(db, limit.try_into().unwrap());

        paginator.fetch_page((page - 1).try_into().unwrap()).await?
    } else if !before.is_empty() {
        let rows = asset::Entity::find()
            .order_by_asc(sort_column)
            .join(
                JoinType::LeftJoin,
                asset::Entity::has_many(asset_creators::Entity).into(),
            )
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
                let asset_data = x.find_related(AssetData).one(db).await.unwrap();

                (x, asset_data)
            });

        let assets = futures::future::join_all(rows).await;
        assets
    } else {
        let rows = asset::Entity::find()
            .order_by_asc(sort_column)
            .join(
                JoinType::LeftJoin,
                asset::Entity::has_many(asset_creators::Entity).into(),
            )
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