pub mod error;
pub mod events;
pub mod parsers;
pub mod utils;
pub mod tasks;

use sea_orm::sea_query::BinOper::In;
use {
    crate::{
        parsers::*,
        utils::{order_instructions, parse_logs},
    },
    futures_util::TryFutureExt,
    messenger::{Messenger, RedisMessenger, ACCOUNT_STREAM, TRANSACTION_STREAM},
    plerkle_serialization::account_info_generated::account_info::root_as_account_info,
    plerkle_serialization::transaction_info_generated::transaction_info::root_as_transaction_info,
    solana_sdk::pubkey::Pubkey,
    sqlx::{self, postgres::PgPoolOptions, Pool, Postgres},
    tokio::sync::mpsc::UnboundedSender,
    serde::Deserialize,
    figment::{Figment, providers::Env},
};
use messenger::MessengerConfig;
use crate::error::IngesterError;
use crate::tasks::{BgTask, TaskManager};

async fn setup_manager<'a, 'b>(
    mut manager: ProgramHandlerManager<'a>,
    pool: Pool<Postgres>,
    task_manager: UnboundedSender<Box<dyn BgTask>>,
) -> ProgramHandlerManager<'a> {
    // Panic if thread cant be made for background tasks
    let bubblegum_parser = BubblegumHandler::new(pool.clone(), task_manager);
    let gummyroll_parser = GummyRollHandler::new(pool.clone());
    manager.register_parser(Box::new(bubblegum_parser));
    manager.register_parser(Box::new(gummyroll_parser));
    manager
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct IngesterConfig {
    messenger_config: MessengerConfig,
    database_url: String,
}


#[tokio::main]
async fn main() {
    println!("Starting DASgester");
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
    let background_task_manager = TaskManager::new("background-tasks".to_string(), pool.clone()).unwrap();
    // Service streams as separate concurrent processes.
    println!("Setting up tasks");
    tasks.push(service_transaction_stream::<RedisMessenger>(pool, background_task_manager.get_sender(), config.messenger_config.clone()).await);
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

async fn service_transaction_stream<T: Messenger>(
    pool: Pool<Postgres>,
    tasks: UnboundedSender<Box<dyn BgTask>>,
    messenger_config: MessengerConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut manager = ProgramHandlerManager::new();

        manager = setup_manager(manager, pool, tasks).await;
        let mut messenger = T::new(messenger_config).await.unwrap();
        println!("Setting up transaction listener");
        loop {
            // This call to messenger.recv() blocks with no timeout until
            // a message is received on the stream.
            if let Ok(data) = messenger.recv(TRANSACTION_STREAM).await {
                handle_transaction(&manager, data).await;
            }
        }
    })
}

async fn service_account_stream<T: Messenger>(
    pool: Pool<Postgres>,
    tasks: UnboundedSender<Box<dyn BgTask>>,
    messenger_config: MessengerConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut manager = ProgramHandlerManager::new();
        let task_manager = TaskManager::new("background-tasks".to_string(), pool.clone()).unwrap();
        manager = setup_manager(manager, pool, tasks).await;
        let mut messenger = T::new(messenger_config).await.unwrap();
        println!("Setting up account listener");
        loop {
            // This call to messenger.recv() blocks with no timeout until
            // a message is received on the stream.
            if let Ok(data) = messenger.recv(ACCOUNT_STREAM).await {
                handle_account(&manager, data).await
            }
        }
    })
}

async fn handle_account(manager: &ProgramHandlerManager<'static>, data: Vec<(i64, &[u8])>) {
    for (_message_id, data) in data {
        // Get root of account info flatbuffers object.
        let account_update = match root_as_account_info(data) {
            Err(err) => {
                println!("Flatbuffers AccountInfo deserialization error: {err}");
                continue;
            }
            Ok(account_update) => account_update,
        };
        let program_id = account_update.owner();
        let parser = manager.match_program(program_id.unwrap());
        match parser {
            Some(p) if p.config().responds_to_account == true => {
                let _ = p.handle_account(&account_update).map_err(|e| {
                    println!("Error in instruction handling {:?}", e);
                    e
                });
            }
            _ => {
                println!(
                    "Program Handler not found for program id {:?}",
                    program_id.map(|p| Pubkey::new(p))
                );
            }
        }
    }
}

async fn handle_transaction(manager: &ProgramHandlerManager<'static>, data: Vec<(i64, &[u8])>) {
    for (message_id, data) in data {
        println!("RECV");
        //TODO -> Dedupe the stream, the stream could have duplicates as a way of ensuring fault tolerance if one validator node goes down.
        //  Possible solution is dedup on the plerkle side but this doesnt follow our principle of getting messages out of the validator asd fast as possible.
        //  Consider a Messenger Implementation detail the deduping of whats in this stream so that
        //  1. only 1 ingest instance picks it up, two the stream coming out of the ingester can be considered deduped

        // Get root of transaction info flatbuffers object.
        let transaction = match root_as_transaction_info(data) {
            Err(err) => {
                println!("Flatbuffers TransactionInfo deserialization error: {err}");
                continue;
            }
            Ok(transaction) => transaction,
        };

        // Get account keys flatbuffers object.
        let keys = match transaction.account_keys() {
            None => {
                println!("Flatbuffers account_keys missing");
                continue;
            }
            Some(keys) => keys,
        };
        // Update metadata associated with the programs that store data in leaves
        let instructions = order_instructions(&transaction);
        let parsed_logs = parse_logs(transaction.log_messages()).unwrap();
        for ((outer_ix, inner_ix), parsed_log) in std::iter::zip(instructions, parsed_logs) {
            // Sanity check that instructions and logs were parsed correctly
            assert_eq!(
                outer_ix.0.key().unwrap(),
                parsed_log.0.to_bytes(),
                "expected {:?}, but program log was {:?}",
                outer_ix.0,
                parsed_log.0
            );

            let (program, instruction) = outer_ix;
            let parser = manager.match_program(program.key().unwrap());
            match parser {
                Some(p) if p.config().responds_to_instruction == true => {
                    let _ = p
                        .handle_instruction(&InstructionBundle {
                            message_id,
                            txn_id: "".to_string(),
                            instruction,
                            inner_ix,
                            keys,
                            instruction_logs: parsed_log.1,
                        })
                        .await
                        .map_err(|e| {
                            // Just for logging
                            println!("Error in instruction handling {:?}", e);
                            e
                        });
                }
                _ => {
                    println!("Program Handler not found for program id {:?}", program);
                }
            }
        }
    }
}
// Associates logs with the given program ID
