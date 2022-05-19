mod bubblegum_indexer;
mod crud_indexer;
mod gummyroll_indexer;
mod utils;

use {
    crate::{
        bubblegum_indexer::handle_bubblegum_instruction,
        crud_indexer::handle_gummyroll_crud_instruction,
        gummyroll_indexer::handle_gummyroll_instruction,
        utils::{filter_events_from_logs, order_instructions},
    },
    flatbuffers::{ForwardsUOffset, Vector},
    lazy_static::lazy_static,
    messenger::{ACCOUNT_STREAM, BLOCK_STREAM, SLOT_STREAM, TRANSACTION_STREAM},
    plerkle::async_redis_messenger::AsyncRedisMessenger,
    plerkle_serialization::transaction_info_generated::transaction_info::root_as_transaction_info,
    regex::Regex,
    solana_sdk::pubkey::Pubkey,
    sqlx::{self, postgres::PgPoolOptions, Pool, Postgres},
};

mod program_ids {
    #![allow(missing_docs)]

    use solana_sdk::pubkeys;
    pubkeys!(
        token_metadata,
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    );
    pubkeys!(
        gummyroll_crud,
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
    );
    pubkeys!(bubblegum, "BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o");
    pubkeys!(gummyroll, "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");
    pubkeys!(token, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    pubkeys!(a_token, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}

#[tokio::main]
async fn main() {
    // Setup Postgres.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://solana:solana@db/solana")
        .await
        .unwrap();

    let mut tasks = vec![];

    // Service streams as separate concurrent processes.
    tasks.push(service_transaction_stream(pool.clone()).await);
    tasks.push(service_stream(ACCOUNT_STREAM, pool.clone()).await);
    tasks.push(service_stream(SLOT_STREAM, pool.clone()).await);
    tasks.push(service_stream(BLOCK_STREAM, pool.clone()).await);

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

async fn service_transaction_stream(pool: Pool<Postgres>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut messenger = AsyncRedisMessenger::new(TRANSACTION_STREAM).await.unwrap();

        loop {
            // This call to messenger.recv() blocks with no timeout until
            // a message is received on the stream.
            if let Ok(data) = messenger.recv().await {
                handle_transaction(data, &pool).await;
            }
        }
    })
}

async fn service_stream(
    stream_key: &'static str,
    _pool: Pool<Postgres>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut messenger = AsyncRedisMessenger::new(stream_key).await.unwrap();

        loop {
            // This call to messenger.recv() blocks with no timeout until
            // a message is received on the stream.
            if let Ok(_data) = messenger.recv().await {
                ()
            }
        }
    })
}

async fn handle_transaction(data: Vec<(i64, &[u8])>, pool: &Pool<Postgres>) {
    for (pid, data) in data {
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
        for (program_instruction, parsed_log) in std::iter::zip(instructions, parsed_logs) {
            // Sanity check that instructions and logs were parsed correctly
            assert!(
                program_instruction.0 == parsed_log.0,
                "expected {:?}, but program log was {:?}",
                program_instruction.0,
                parsed_log.0
            );

            match program_instruction {
                (program, _instruction) if program == program_ids::gummyroll() => {
                    handle_gummyroll_instruction(&parsed_log.1, pid, pool)
                        .await
                        .unwrap();
                }
                (program, instruction) if program == program_ids::gummyroll_crud() => {
                    handle_gummyroll_crud_instruction(&instruction, &keys, pid, pool)
                        .await
                        .unwrap();
                }
                (program, instruction) if program == program_ids::bubblegum() => {
                    handle_bubblegum_instruction(&instruction, &parsed_log.1, &keys, pid, pool)
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    }
}
// Associates logs with the given program ID
fn parse_logs(
    log_messages: Option<Vector<ForwardsUOffset<&str>>>,
) -> Result<Vec<(Pubkey, Vec<String>)>, ()> {
    lazy_static! {
        static ref PLRE: Regex = Regex::new(r"Program (\w*) invoke \[(\d)\]").unwrap();
    }
    let mut program_logs: Vec<(Pubkey, Vec<String>)> = vec![];

    match log_messages {
        Some(logs) => {
            for log in logs {
                let captures = PLRE.captures(log);
                let pubkey_bytes = captures
                    .and_then(|c| c.get(1))
                    .map(|c| bs58::decode(&c.as_str()).into_vec().unwrap());

                match pubkey_bytes {
                    None => {
                        let last_program_log = program_logs.last_mut().unwrap();
                        (*last_program_log).1.push(log.parse().unwrap());
                    }
                    Some(bytes) => {
                        program_logs.push((Pubkey::new(&bytes), vec![]));
                    }
                }
            }
            Ok(program_logs)
        }
        None => {
            println!("No logs found in transaction info!");
            Err(())
        }
    }
}
