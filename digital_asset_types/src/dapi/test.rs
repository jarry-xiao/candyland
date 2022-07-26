
   
   #![cfg(feature = "mock")]
   
   use sea_orm::{
        entity::prelude::*, entity::*, tests_cfg::*,
        DatabaseBackend, MockDatabase, MockExecResult, Transaction,
    };
use solana_sdk::signature::Keypair;
use bubblegum::state::metaplex_adapter::MetadataArgs;
use digital_asset_types::adapter::{TokenStandard, UseMethod, Uses, Creator, TokenProgramVersion};


    
    #[async_std::test]
    async fn test_get_asset() -> Result<(), DbErr> {
       let db = MockDatabase::new(DatabaseBackend::Postgres).append_query_results(vec![]).append_exec_results(vec![]).into_connection();

       let id = Keypair::new().pubkey();
       let owner = Keypair::new().pubkey();
       let update_authority = Keypair::new().pubkey();

       let metadata_1 =  MetadataArgs {
        name:"Test #1",
              symbol: "BUBBLE",
              uri: Keypair.generate().publicKey.toBase58(),
              sellerFeeBasisPoints: 100,
              primary_sale_happened: true,
              is_mutable: true,
              edition_nonce: None,
              token_standard: null,
              collection: None,
              uses: None,
              token_program_version: TokenProgramVersion::Original,
              creators: vec![
                Creator { address: Keypair::new().pubkey(), share: 100, verified: true },
              ].to_vec()};

          let chain_data_1 = ChainDataV1 {
        name: metadata_1.name,
        symbol: metadata_1.symbol,
        edition_nonce: metadata_1.edition_nonce,
        primary_sale_happened: metadata_1.primary_sale_happened,
        token_standard: metadata_1
            .token_standard
            .and_then(|ts| TokenStandard::from_u8(ts as u8)),
        uses: None,
    };

let chain_data_json = serde_json::to_value(chain_data).unwrap();

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
  
    let insert_result = asset_data::Entity::insert(asset_1).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);

       let asset_1 =     asset::ActiveModel {
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
        /// Get gummy roll seq
        royalty_target_type: Set(RoyaltyTargetType::Creators),
        royalty_target: Set(None),
        royalty_amount: Set(metadata_1.seller_fee_basis_points as i32), //basis points
        chain_data_id: Set(Some(data_1.id)),
        ..Default::default()
    };


    
   let insert_result = asset::Entity::insert(asset_1).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);
    
    if metadata_1.creators.len() > 0 {
        let mut creators = Vec::with_capacity(metadata_1.creators.len());
        for c in metadata_1.creators {
            creators.push(asset_creators::ActiveModel {
                asset_id: Set(id.to_bytes().to_vec()),
                creator: Set(c.address.to_bytes().to_vec()),
                share: Set(c.share as i32),
                verified: Set(c.verified),
                ..Default::default()
            });
        }
        asset_creators::Entity::insert_many(creators)
            .exec(&db)
            .await?;
    }

     let authority_1=   asset_authority::ActiveModel {
        asset_id: Set(id.to_bytes().to_vec()),
        authority: Set(update_authority.to_bytes().to_vec()),
        ..Default::default()
    };

       let insert_result = asset_authority::Entity::insert(authority_1).exec(&db).await?;
        assert_eq!(insert_result.last_insert_id, 1);
    

        Ok(())
    }
