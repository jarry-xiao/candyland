#[cfg(test)]
mod get_assets_by_creator {
    use sea_orm::{
        entity::prelude::*, entity::*, DatabaseBackend, JsonValue, MockDatabase, MockExecResult,
    };
    use solana_sdk::{signature::Keypair, signer::Signer};

    use crate::{
        adapter::{Creator, TokenProgramVersion, TokenStandard},
        dao::{
            asset, asset_authority, asset_creators, asset_data,
            prelude::AssetData,
            sea_orm_active_enums::{ChainMutability, Mutability, OwnerType, RoyaltyTargetType},
        },
        json::ChainDataV1,
        tests::MetadataArgs,
    };

    #[cfg(feature = "mock")]
    #[tokio::test]
    async fn get_assets_by_creator() -> Result<(), DbErr> {
        let id_1 = Keypair::new().pubkey();
        let owner_1 = Keypair::new().pubkey();
        let update_authority_1 = Keypair::new().pubkey();
        let creator_1 = Keypair::new().pubkey();
        let uri_1 = Keypair::new().pubkey();

        let id_2 = Keypair::new().pubkey();
        let owner_2 = Keypair::new().pubkey();
        let update_authority_2 = Keypair::new().pubkey();
        let creator_2 = Keypair::new().pubkey();
        let uri_2 = Keypair::new().pubkey();

        let id_3 = Keypair::new().pubkey();
        let update_authority_3 = Keypair::new().pubkey();
        let creator_3 = Keypair::new().pubkey();
        let uri_3 = Keypair::new().pubkey();

        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![asset_data::Model {
                id: 1,
                chain_data_mutability: ChainMutability::Mutable,
                schema_version: 1,
                chain_data: serde_json::to_value(ChainDataV1 {
                    name: String::from("Test #1"),
                    symbol: String::from("BUBBLE"),
                    edition_nonce: None,
                    primary_sale_happened: true,
                    token_standard: Some(TokenStandard::NonFungible),
                    uses: None,
                })
                .unwrap(),
                metadata_url: Keypair::new().pubkey().to_string(),
                metadata_mutability: Mutability::Mutable,
                metadata: JsonValue::String("processing".to_string()),
            }]])
            .append_query_results(vec![vec![asset::Model {
                id: id_1.to_bytes().to_vec(),
                owner: owner_1.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(1),
                burnt: false,
                created_at: None,
            }]])
            .append_query_results(vec![vec![asset_creators::Model {
                id: 1,
                asset_id: id_1.to_bytes().to_vec(),
                creator: creator_1.to_bytes().to_vec(),
                share: 100,
                verified: true,
            }]])
            .append_query_results(vec![vec![asset_authority::Model {
                asset_id: id_1.to_bytes().to_vec(),
                authority: update_authority_1.to_bytes().to_vec(),
                id: 1,
                scopes: None,
            }]])
            .append_query_results(vec![vec![asset_data::Model {
                id: 2,
                chain_data_mutability: ChainMutability::Mutable,
                schema_version: 1,
                chain_data: serde_json::to_value(ChainDataV1 {
                    name: String::from("Test #2"),
                    symbol: String::from("BUBBLE"),
                    edition_nonce: None,
                    primary_sale_happened: true,
                    token_standard: Some(TokenStandard::NonFungible),
                    uses: None,
                })
                .unwrap(),
                metadata_url: Keypair::new().pubkey().to_string(),
                metadata_mutability: Mutability::Mutable,
                metadata: JsonValue::String("processing".to_string()),
            }]])
            .append_query_results(vec![vec![asset::Model {
                id: id_2.to_bytes().to_vec(),
                owner: owner_2.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(2),
                burnt: false,
                created_at: None,
            }]])
            .append_query_results(vec![vec![asset_creators::Model {
                id: 2,
                asset_id: id_2.to_bytes().to_vec(),
                creator: creator_2.to_bytes().to_vec(),
                share: 100,
                verified: true,
            }]])
            .append_query_results(vec![vec![asset_authority::Model {
                asset_id: id_2.to_bytes().to_vec(),
                authority: update_authority_2.to_bytes().to_vec(),
                id: 2,
                scopes: None,
            }]])
            .append_query_results(vec![vec![asset_data::Model {
                id: 3,
                chain_data_mutability: ChainMutability::Mutable,
                schema_version: 1,
                chain_data: serde_json::to_value(ChainDataV1 {
                    name: String::from("Test #3"),
                    symbol: String::from("BUBBLE"),
                    edition_nonce: None,
                    primary_sale_happened: true,
                    token_standard: Some(TokenStandard::NonFungible),
                    uses: None,
                })
                .unwrap(),
                metadata_url: Keypair::new().pubkey().to_string(),
                metadata_mutability: Mutability::Mutable,
                metadata: JsonValue::String("processing".to_string()),
            }]])
            .append_query_results(vec![vec![asset::Model {
                id: id_3.to_bytes().to_vec(),
                owner: owner_2.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(3),
                burnt: false,
                created_at: None,
            }]])
            .append_query_results(vec![vec![asset_creators::Model {
                id: 3,
                asset_id: id_3.to_bytes().to_vec(),
                creator: creator_2.to_bytes().to_vec(),
                share: 100,
                verified: true,
            }]])
            .append_query_results(vec![vec![asset_authority::Model {
                asset_id: id_3.to_bytes().to_vec(),
                authority: update_authority_3.to_bytes().to_vec(),
                id: 3,
                scopes: None,
            }]])
            .append_query_results(vec![vec![
                (
                    asset::Model {
                        id: id_2.to_bytes().to_vec(),
                        owner: owner_2.to_bytes().to_vec(),
                        owner_type: OwnerType::Single,
                        delegate: None,
                        frozen: false,
                        supply: 1,
                        supply_mint: None,
                        compressed: true,
                        compressible: false,
                        tree_id: None,
                        specification_version: 1,
                        nonce: (0 as i64),
                        leaf: None,
                        royalty_target_type: RoyaltyTargetType::Creators,
                        royalty_target: None,
                        royalty_amount: 100, //basis points
                        chain_data_id: Some(2),
                        burnt: false,
                        created_at: None,
                    },
                    asset_data::Model {
                        id: 2,
                        chain_data_mutability: ChainMutability::Mutable,
                        schema_version: 1,
                        chain_data: serde_json::to_value(ChainDataV1 {
                            name: String::from("Test #2"),
                            symbol: String::from("BUBBLE"),
                            edition_nonce: None,
                            primary_sale_happened: true,
                            token_standard: Some(TokenStandard::NonFungible),
                            uses: None,
                        })
                        .unwrap(),
                        metadata_url: uri_2.to_string(),
                        metadata_mutability: Mutability::Mutable,
                        metadata: JsonValue::String("processing".to_string()),
                    },
                ),
                (
                    asset::Model {
                        id: id_3.to_bytes().to_vec(),
                        owner: owner_2.to_bytes().to_vec(),
                        owner_type: OwnerType::Single,
                        delegate: None,
                        frozen: false,
                        supply: 1,
                        supply_mint: None,
                        compressed: true,
                        compressible: false,
                        tree_id: None,
                        specification_version: 1,
                        nonce: (0 as i64),
                        leaf: None,
                        royalty_target_type: RoyaltyTargetType::Creators,
                        royalty_target: None,
                        royalty_amount: 100, //basis points
                        chain_data_id: Some(3),
                        burnt: false,
                        created_at: None,
                    },
                    asset_data::Model {
                        id: 3,
                        chain_data_mutability: ChainMutability::Mutable,
                        schema_version: 1,
                        chain_data: serde_json::to_value(ChainDataV1 {
                            name: String::from("Test #3"),
                            symbol: String::from("BUBBLE"),
                            edition_nonce: None,
                            primary_sale_happened: true,
                            token_standard: Some(TokenStandard::NonFungible),
                            uses: None,
                        })
                        .unwrap(),
                        metadata_url: uri_3.to_string(),
                        metadata_mutability: Mutability::Mutable,
                        metadata: JsonValue::String("processing".to_string()),
                    },
                ),
            ]])
            .into_connection();

        let metadata_1 = MetadataArgs {
            name: String::from("Test #1"),
            symbol: String::from("BUBBLE"),
            uri: uri_1.to_string(),
            primary_sale_happened: true,
            is_mutable: true,
            edition_nonce: None,
            token_standard: Some(TokenStandard::NonFungible),
            collection: None,
            uses: None,
            token_program_version: TokenProgramVersion::Original,
            creators: vec![Creator {
                address: creator_1,
                share: 100,
                verified: true,
            }]
            .to_vec(),
            seller_fee_basis_points: 100,
        };

        let chain_data_1 = ChainDataV1 {
            name: metadata_1.name,
            symbol: metadata_1.symbol,
            edition_nonce: metadata_1.edition_nonce,
            primary_sale_happened: metadata_1.primary_sale_happened,
            token_standard: metadata_1.token_standard,
            uses: None,
        };

        let chain_data_json = serde_json::to_value(chain_data_1).unwrap();

        let chain_mutability = match metadata_1.is_mutable {
            true => ChainMutability::Mutable,
            false => ChainMutability::Immutable,
        };

        let data_1 = asset_data::ActiveModel {
            chain_data_mutability: Set(chain_mutability),
            schema_version: Set(1),
            chain_data: Set(chain_data_json),
            metadata_url: Set(metadata_1.uri),
            metadata: Set(JsonValue::String("processing".to_string())),
            metadata_mutability: Set(Mutability::Mutable),
            ..Default::default()
        };

        let insert_result = asset_data::Entity::insert(data_1).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);

        let asset_1 = asset::ActiveModel {
            id: Set(id_1.to_bytes().to_vec()),
            owner: Set(owner_1.to_bytes().to_vec()),
            owner_type: Set(OwnerType::Single),
            delegate: Set(None),
            frozen: Set(false),
            supply: Set(1),
            supply_mint: Set(None),
            compressed: Set(true),
            compressible: Set(false),
            tree_id: Set(None),
            specification_version: Set(1),
            nonce: Set(0 as i64),
            leaf: Set(None),
            royalty_target_type: Set(RoyaltyTargetType::Creators),
            royalty_target: Set(None),
            royalty_amount: Set(metadata_1.seller_fee_basis_points as i32), //basis points
            chain_data_id: Set(Some(insert_result.last_insert_id)),
            ..Default::default()
        };

        assert_eq!(
            asset_1.insert(&db).await?,
            asset::Model {
                id: id_1.to_bytes().to_vec(),
                owner: owner_1.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(1),
                burnt: false,
                created_at: None,
            }
        );

        let creator = asset_creators::ActiveModel {
            asset_id: Set(id_1.to_bytes().to_vec()),
            creator: Set(metadata_1.creators[0].address.to_bytes().to_vec()),
            share: Set(metadata_1.creators[0].share as i32),
            verified: Set(metadata_1.creators[0].verified),
            ..Default::default()
        };

        let insert_result = asset_creators::Entity::insert(creator).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);

        let authority_1 = asset_authority::ActiveModel {
            asset_id: Set(id_1.to_bytes().to_vec()),
            authority: Set(update_authority_1.to_bytes().to_vec()),
            ..Default::default()
        };

        let insert_result = asset_authority::Entity::insert(authority_1)
            .exec(&db)
            .await?;
        assert_eq!(insert_result.last_insert_id, 1);

        let metadata_2 = MetadataArgs {
            name: String::from("Test #2"),
            symbol: String::from("BUBBLE"),
            uri: uri_2.to_string(),
            primary_sale_happened: true,
            is_mutable: true,
            edition_nonce: None,
            token_standard: Some(TokenStandard::NonFungible),
            collection: None,
            uses: None,
            token_program_version: TokenProgramVersion::Original,
            creators: vec![Creator {
                address: creator_2,
                share: 100,
                verified: true,
            }]
            .to_vec(),
            seller_fee_basis_points: 100,
        };

        let chain_data_2 = ChainDataV1 {
            name: metadata_2.name,
            symbol: metadata_2.symbol,
            edition_nonce: metadata_2.edition_nonce,
            primary_sale_happened: metadata_2.primary_sale_happened,
            token_standard: metadata_2.token_standard,
            uses: None,
        };

        let chain_data_json = serde_json::to_value(chain_data_2).unwrap();

        let chain_mutability = match metadata_2.is_mutable {
            true => ChainMutability::Mutable,
            false => ChainMutability::Immutable,
        };

        let data_2 = asset_data::ActiveModel {
            chain_data_mutability: Set(chain_mutability),
            schema_version: Set(1),
            chain_data: Set(chain_data_json),
            metadata_url: Set(metadata_2.uri),
            metadata: Set(JsonValue::String("processing".to_string())),
            metadata_mutability: Set(Mutability::Mutable),
            ..Default::default()
        };

        let insert_result = asset_data::Entity::insert(data_2).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 2);

        let asset_2 = asset::ActiveModel {
            id: Set(id_2.to_bytes().to_vec()),
            owner: Set(owner_2.to_bytes().to_vec()),
            owner_type: Set(OwnerType::Single),
            delegate: Set(None),
            frozen: Set(false),
            supply: Set(1),
            supply_mint: Set(None),
            compressed: Set(true),
            compressible: Set(false),
            tree_id: Set(None),
            specification_version: Set(1),
            nonce: Set(0 as i64),
            leaf: Set(None),
            royalty_target_type: Set(RoyaltyTargetType::Creators),
            royalty_target: Set(None),
            royalty_amount: Set(metadata_2.seller_fee_basis_points as i32), //basis points
            chain_data_id: Set(Some(insert_result.last_insert_id)),
            ..Default::default()
        };

        assert_eq!(
            asset_2.insert(&db).await?,
            asset::Model {
                id: id_2.to_bytes().to_vec(),
                owner: owner_2.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(2),
                burnt: false,
                created_at: None,
            }
        );

        let creator = asset_creators::ActiveModel {
            asset_id: Set(id_2.to_bytes().to_vec()),
            creator: Set(metadata_2.creators[0].address.to_bytes().to_vec()),
            share: Set(metadata_2.creators[0].share as i32),
            verified: Set(metadata_2.creators[0].verified),
            ..Default::default()
        };

        let insert_result = asset_creators::Entity::insert(creator).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 2);

        let authority_2 = asset_authority::ActiveModel {
            asset_id: Set(id_2.to_bytes().to_vec()),
            authority: Set(update_authority_2.to_bytes().to_vec()),
            ..Default::default()
        };

        let insert_result = asset_authority::Entity::insert(authority_2)
            .exec(&db)
            .await?;
        assert_eq!(insert_result.last_insert_id, 2);

        let metadata_3 = MetadataArgs {
            name: String::from("Test #3"),
            symbol: String::from("BUBBLE"),
            uri: uri_3.to_string(),
            primary_sale_happened: true,
            is_mutable: true,
            edition_nonce: None,
            token_standard: Some(TokenStandard::NonFungible),
            collection: None,
            uses: None,
            token_program_version: TokenProgramVersion::Original,
            creators: vec![Creator {
                address: creator_2,
                share: 100,
                verified: true,
            }]
            .to_vec(),
            seller_fee_basis_points: 100,
        };

        let chain_data_3 = ChainDataV1 {
            name: metadata_3.name,
            symbol: metadata_3.symbol,
            edition_nonce: metadata_3.edition_nonce,
            primary_sale_happened: metadata_3.primary_sale_happened,
            token_standard: metadata_3.token_standard,
            uses: None,
        };

        let chain_data_json = serde_json::to_value(chain_data_3).unwrap();

        let chain_mutability = match metadata_3.is_mutable {
            true => ChainMutability::Mutable,
            false => ChainMutability::Immutable,
        };

        let data_3 = asset_data::ActiveModel {
            chain_data_mutability: Set(chain_mutability),
            schema_version: Set(1),
            chain_data: Set(chain_data_json),
            metadata_url: Set(metadata_3.uri),
            metadata: Set(JsonValue::String("processing".to_string())),
            metadata_mutability: Set(Mutability::Mutable),
            ..Default::default()
        };

        let insert_result = asset_data::Entity::insert(data_3).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 3);

        let asset_3 = asset::ActiveModel {
            id: Set(id_3.to_bytes().to_vec()),
            owner: Set(owner_2.to_bytes().to_vec()),
            owner_type: Set(OwnerType::Single),
            delegate: Set(None),
            frozen: Set(false),
            supply: Set(1),
            supply_mint: Set(None),
            compressed: Set(true),
            compressible: Set(false),
            tree_id: Set(None),
            specification_version: Set(1),
            nonce: Set(0 as i64),
            leaf: Set(None),
            royalty_target_type: Set(RoyaltyTargetType::Creators),
            royalty_target: Set(None),
            royalty_amount: Set(metadata_3.seller_fee_basis_points as i32), //basis points
            chain_data_id: Set(Some(insert_result.last_insert_id)),
            ..Default::default()
        };

        assert_eq!(
            asset_3.insert(&db).await?,
            asset::Model {
                id: id_3.to_bytes().to_vec(),
                owner: owner_2.to_bytes().to_vec(),
                owner_type: OwnerType::Single,
                delegate: None,
                frozen: false,
                supply: 1,
                supply_mint: None,
                compressed: true,
                compressible: false,
                tree_id: None,
                specification_version: 1,
                nonce: (0 as i64),
                leaf: None,
                royalty_target_type: RoyaltyTargetType::Creators,
                royalty_target: None,
                royalty_amount: 100, //basis points
                chain_data_id: Some(3),
                burnt: false,
                created_at: None,
            }
        );

        let creator = asset_creators::ActiveModel {
            asset_id: Set(id_3.to_bytes().to_vec()),
            creator: Set(metadata_3.creators[0].address.to_bytes().to_vec()),
            share: Set(metadata_3.creators[0].share as i32),
            verified: Set(metadata_3.creators[0].verified),
            ..Default::default()
        };

        let insert_result = asset_creators::Entity::insert(creator).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 3);

        let authority_3 = asset_authority::ActiveModel {
            asset_id: Set(id_3.to_bytes().to_vec()),
            authority: Set(update_authority_3.to_bytes().to_vec()),
            ..Default::default()
        };

        let insert_result = asset_authority::Entity::insert(authority_3)
            .exec(&db)
            .await?;
        assert_eq!(insert_result.last_insert_id, 3);

        assert_eq!(
            asset_creators::Entity::find()
                .filter(
                Condition::any()
                    .add(asset_creators::Column::Creator.eq(creator_2.to_bytes().to_vec())), // .add(asset_creators::Column::Creator.eq(creator_expression[1].clone())),
            )
                .find_also_related(Asset)
                .all(&db)
                .await?,
            vec![
                (
                    asset::Model {
                        id: id_2.to_bytes().to_vec(),
                        owner: owner_2.to_bytes().to_vec(),
                        owner_type: OwnerType::Single,
                        delegate: None,
                        frozen: false,
                        supply: 1,
                        supply_mint: None,
                        compressed: true,
                        compressible: false,
                        tree_id: None,
                        specification_version: 1,
                        nonce: (0 as i64),
                        leaf: None,
                        royalty_target_type: RoyaltyTargetType::Creators,
                        royalty_target: None,
                        royalty_amount: 100, //basis points
                        chain_data_id: Some(2),
                        burnt: false,
                        created_at: None,
                    },
                    Some(asset_data::Model {
                        id: 2,
                        chain_data_mutability: ChainMutability::Mutable,
                        schema_version: 1,
                        chain_data: serde_json::to_value(ChainDataV1 {
                            name: String::from("Test #2"),
                            symbol: String::from("BUBBLE"),
                            edition_nonce: None,
                            primary_sale_happened: true,
                            token_standard: Some(TokenStandard::NonFungible),
                            uses: None,
                        })
                        .unwrap(),
                        metadata_url: uri_2.to_string(),
                        metadata_mutability: Mutability::Mutable,
                        metadata: JsonValue::String("processing".to_string()),
                    })
                ),
                (
                    asset::Model {
                        id: id_3.to_bytes().to_vec(),
                        owner: owner_2.to_bytes().to_vec(),
                        owner_type: OwnerType::Single,
                        delegate: None,
                        frozen: false,
                        supply: 1,
                        supply_mint: None,
                        compressed: true,
                        compressible: false,
                        tree_id: None,
                        specification_version: 1,
                        nonce: (0 as i64),
                        leaf: None,
                        royalty_target_type: RoyaltyTargetType::Creators,
                        royalty_target: None,
                        royalty_amount: 100, //basis points
                        chain_data_id: Some(3),
                        burnt: false,
                        created_at: None,
                    },
                    Some(asset_data::Model {
                        id: 3,
                        chain_data_mutability: ChainMutability::Mutable,
                        schema_version: 1,
                        chain_data: serde_json::to_value(ChainDataV1 {
                            name: String::from("Test #3"),
                            symbol: String::from("BUBBLE"),
                            edition_nonce: None,
                            primary_sale_happened: true,
                            token_standard: Some(TokenStandard::NonFungible),
                            uses: None,
                        })
                        .unwrap(),
                        metadata_url: uri_3.to_string(),
                        metadata_mutability: Mutability::Mutable,
                        metadata: JsonValue::String("processing".to_string()),
                    })
                )
            ]
        );
        Ok(())
    }
}
