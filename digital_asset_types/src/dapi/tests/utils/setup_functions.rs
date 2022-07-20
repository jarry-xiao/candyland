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

pub fn bubble_gum_program_test() -> ProgramTest {
    let mut program = ProgramTest::new("bubblegum", bubblegum::id(), None);
    program.add_program("bubblegum", bubblegum::id(), None);
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
    context: &mut ProgramTestContext,

) -> StdResult<Pubkey, TransportError> {
    let accounts = bubblegum::accounts::MintV1 {
  
    }
    .to_account_metas(None);

    let data = bubblegum::instruction::MintV1 {
       
    }
    .data();

    let instruction = Instruction {
        program_id: bubblegum::id(),
        data,
        accounts,
    };

    let tx = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer_wallet.pubkey()),
        &[payer_wallet],
        context.last_blockhash,
    );

    context
        .banks_client
        .process_transaction(tx)
        .await?
}
