use std::io;

use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program, sysvar,
};
use anchor_lang::*;

use std::result::Result as StdResult;

use solana_program_test::*;
use solana_sdk::{instruction::Instruction, transaction::Transaction, transport::TransportError};
use spl_associated_token_account::get_associated_token_address;

const BUBBLEGUM_PROGRAM_ID: Pubkey = pubkey!("BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY");

pub fn bubble_gum_program_test() -> ProgramTest {
    let mut program = ProgramTest::new("bubblegum", BUBBLEGUM_PROGRAM_ID, None);
    program.add_program("bubblegum", BUBBLEGUM_PROGRAM_ID, None);
    program
}

pub fn ingester_setup() -> () {
        let config: IngesterConfig = Figment::new()
        .join(Env::prefixed("INGESTER_"))
        .extract()
        .map_err(|config_error| IngesterError::ConfigurationError { msg: format!("{}", config_error) }).unwrap();

           // Setup Postgres.
    let mut tasks = vec![];
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&*config.database_url)
        .await
        .unwrap();
    let background_task_manager =
        TaskManager::new("background-tasks".to_string(), pool.clone()).unwrap();
    // Service streams as separate concurrent processes.
    tasks.push(
        service_transaction_stream::<RedisMessenger>(
            pool.clone(),
            background_task_manager.get_sender(),
            config.messenger_config.clone(),
        )
        .await,
    );
    // Start up backfiller process.
    tasks.push(backfiller(pool.clone()).await);
    // Wait for ctrl-c.
    match tokio::signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            println!("Unable to listen for shutdown signal: {}", err);
            // We also shut down in case of error.
        }
    }

    // Kill all tasks.
    for task in tasks {
        task.abort();
    }
}

pub async fn create_and_insert_asset(
    db: &DatabaseConnection,
    metadata: MetadataArgs,
    id: Pubkey,
    owner: Pubkey,
) -> i64 {
    println!("BGUM: MINT");
    // Printing metadata instruction arguments for debugging
    println!(
        "\tMetadata info: {} {} {} {} {}",
        id.to_string(),
        &metadata.name,
        metadata.seller_fee_basis_points,
        metadata.primary_sale_happened,
        metadata.is_mutable,
    );
    let chain_data = ChainDataV1 {
        name: metadata.name,
        symbol: metadata.symbol,
        edition_nonce: metadata.edition_nonce,
        primary_sale_happened: metadata.primary_sale_happened,
        token_standard: metadata
            .token_standard
            .and_then(|ts| TokenStandard::from_u8(ts as u8)),
        uses: None,
    };

    let chain_data_json = serde_json::to_value(chain_data).unwrap();

    let chain_mutability = match metadata.is_mutable {
        true => ChainMutability::Mutable,
        false => ChainMutability::Immutable,
    };

    let data = asset_data::ActiveModel {
        chain_data_mutability: Set(chain_mutability),
        schema_version: Set(1),
        chain_data: Set(chain_data_json),
        metadata_url: Set(metadata.uri),
        metadata: Set(JsonValue::String("processing".to_string())),
        metadata_mutability: Set(Mutability::Mutable),
        ..Default::default()
    }
    .insert(txn)
    .await
    .unwrap();

    asset::ActiveModel {
        id: Set(id.to_bytes().to_vec()),
        owner: Set(owner),
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
        royalty_amount: Set(metadata.seller_fee_basis_points as i32), //basis points
        chain_data_id: Set(Some(data.id)),
        ..Default::default()
    }
    .insert(txn)
    .await
    .unwrap();

    if metadata.creators.len() > 0 {
        let mut creators = Vec::with_capacity(metadata.creators.len());
        for c in metadata.creators {
            creators.push(asset_creators::ActiveModel {
                asset_id: Set(id.to_bytes().to_vec()),
                creator: Set(c.address.to_bytes().to_vec()),
                share: Set(c.share as i32),
                verified: Set(c.verified),
                ..Default::default()
            });
        }
        asset_creators::Entity::insert_many(creators)
            .exec(txn)
            .await
            .map_err(|txn_err| IngesterError::StorageWriteError(txn_err.to_string()))?;
    }
    asset_authority::ActiveModel {
        asset_id: Set(id.to_bytes().to_vec()),
        authority: Set(update_authority),
        ..Default::default()
    }
    .insert(txn)
    .await
    .unwrap();

    data.id
}
