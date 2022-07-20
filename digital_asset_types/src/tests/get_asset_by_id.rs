#[cfg(test)]
mod starter {
    use sea_orm::{
        entity::prelude::*, entity::*, DatabaseBackend, JsonValue, MockDatabase, MockExecResult,
    };
    use solana_sdk::{signature::Keypair, signer::Signer};

    use crate::{
        adapter::{Collection, Creator, TokenProgramVersion, TokenStandard, Uses},
        dao::{
            asset, asset_authority, asset_creators, asset_data,
            prelude::AssetData,
            sea_orm_active_enums::{ChainMutability, Mutability, OwnerType, RoyaltyTargetType},
        },
        json::ChainDataV1,
    };

    pub struct MetadataArgs {
        /// The name of the asset
        pub name: String,
        /// The symbol for the asset
        pub symbol: String,
        /// URI pointing to JSON representing the asset
        pub uri: String,
        /// Royalty basis points that goes to creators in secondary sales (0-10000)
        pub seller_fee_basis_points: u16,
        // Immutable, once flipped, all sales of this metadata are considered secondary.
        pub primary_sale_happened: bool,
        // Whether or not the data struct is mutable, default is not
        pub is_mutable: bool,
        /// nonce for easy calculation of editions, if present
        pub edition_nonce: Option<u8>,
        /// Since we cannot easily change Metadata, we add the new DataV2 fields here at the end.
        pub token_standard: Option<TokenStandard>,
        /// Collection
        pub collection: Option<Collection>,
        /// Uses
        pub uses: Option<Uses>,
        pub token_program_version: TokenProgramVersion,
        pub creators: Vec<Creator>,
    }

    #[cfg(feature = "mock")]
    #[tokio::test]
    async fn get_asset_by_id() -> Result<(), DbErr> {
        let id = Keypair::new().pubkey();
        let owner = Keypair::new().pubkey();
        let update_authority = Keypair::new().pubkey();
        let creator_1 = Keypair::new().pubkey();
        let uri = Keypair::new().pubkey();

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
                id: id.to_bytes().to_vec(),
                owner: owner.to_bytes().to_vec(),
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
                asset_id: id.to_bytes().to_vec(),
                creator: creator_1.to_bytes().to_vec(),
                share: 100,
                verified: true,
            }]])
            .append_query_results(vec![vec![asset_authority::Model {
                asset_id: id.to_bytes().to_vec(),
                authority: update_authority.to_bytes().to_vec(),
                id: 1,
                scopes: None,
            }]])
            .append_query_results(vec![vec![(
                asset::Model {
                    id: id.to_bytes().to_vec(),
                    owner: owner.to_bytes().to_vec(),
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
                },
                asset_data::Model {
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
                    metadata_url: uri.to_string(),
                    metadata_mutability: Mutability::Mutable,
                    metadata: JsonValue::String("processing".to_string()),
                },
            )]])
            .append_exec_results(vec![MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .into_connection();

        let metadata_1 = MetadataArgs {
            name: String::from("Test #1"),
            symbol: String::from("BUBBLE"),
            uri: uri.to_string(),
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
            id: Set(id.to_bytes().to_vec()),
            owner: Set(owner.to_bytes().to_vec()),
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
                id: id.to_bytes().to_vec(),
                owner: owner.to_bytes().to_vec(),
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
            asset_id: Set(id.to_bytes().to_vec()),
            creator: Set(metadata_1.creators[0].address.to_bytes().to_vec()),
            share: Set(metadata_1.creators[0].share as i32),
            verified: Set(metadata_1.creators[0].verified),
            ..Default::default()
        };

        let insert_result = asset_creators::Entity::insert(creator).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);

        let authority_1 = asset_authority::ActiveModel {
            asset_id: Set(id.to_bytes().to_vec()),
            authority: Set(update_authority.to_bytes().to_vec()),
            ..Default::default()
        };

        let insert_result = asset_authority::Entity::insert(authority_1)
            .exec(&db)
            .await?;
        assert_eq!(insert_result.last_insert_id, 1);

        assert_eq!(
            asset::Entity::find_by_id(id.to_bytes().to_vec())
                .find_also_related(AssetData)
                .one(&db)
                .await?,
            Some((
                asset::Model {
                    id: id.to_bytes().to_vec(),
                    owner: owner.to_bytes().to_vec(),
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
                },
                Some(asset_data::Model {
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
                    metadata_url: uri.to_string(),
                    metadata_mutability: Mutability::Mutable,
                    metadata: JsonValue::String("processing".to_string()),
                })
            ))
        );

        Ok(())
    }
}
