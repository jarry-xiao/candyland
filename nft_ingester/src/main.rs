pub mod error;
pub mod events;
pub mod parsers;
pub mod tasks;
pub mod utils;

<<<<<<< HEAD
use crate::error::IngesterError;
use crate::tasks::{BgTask, TaskManager};
use messenger::MessengerConfig;
use sea_orm::sea_query::BinOper::In;
=======
use cadence_macros::statsd_time;
use chrono::Utc;
>>>>>>> origin/k8s
use {
    crate::{
        parsers::*,
        utils::{order_instructions, parse_logs},
    },
    digital_asset_types::dao::backfill_items::{self, Model},
    figment::{providers::Env, Figment},
    futures_util::TryFutureExt,
    hex::ToHex,
    messenger::{Messenger, RedisMessenger, ACCOUNT_STREAM, TRANSACTION_STREAM},
    plerkle_serialization::account_info_generated::account_info::root_as_account_info,
    plerkle_serialization::transaction_info_generated::transaction_info::root_as_transaction_info,
    sea_orm::{
        entity::*,
        query::*,
        sea_query::{Expr, Query},
        DatabaseConnection, DbBackend, DbErr, FromQueryResult, SqlxPostgresConnector,
    },
    serde::Deserialize,
    solana_sdk::pubkey::Pubkey,
    sqlx::{self, postgres::PgPoolOptions, Pool, Postgres},
    tokio::sync::mpsc::UnboundedSender,
<<<<<<< HEAD
};
=======
    serde::Deserialize,
    figment::{Figment, providers::Env},
    cadence_macros::{
        set_global_default,
        statsd_count,
    },
    cadence::{BufferedUdpMetricSink, QueuingMetricSink, StatsdClient},
    std::net::UdpSocket,
};

use messenger::MessengerConfig;
use crate::error::IngesterError;
use crate::tasks::{BgTask, TaskManager};
>>>>>>> origin/k8s

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
    pub messenger_config: MessengerConfig,
    pub database_url: String,
    pub metrics_port: u16,
    pub metrics_host: String,
}

fn setup_metrics(config: &IngesterConfig) {
    let uri = config.metrics_host.clone();
    let port = config.metrics_port.clone();
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.set_nonblocking(true).unwrap();
    let host = (uri, port);
    let udp_sink = BufferedUdpMetricSink::from(host, socket).unwrap();
    let queuing_sink = QueuingMetricSink::from(udp_sink);
    let client = StatsdClient::from_sink("das_ingester", queuing_sink);
    set_global_default(client);
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
    let background_task_manager =
        TaskManager::new("background-tasks".to_string(), pool.clone()).unwrap();
    // Service streams as separate concurrent processes.
<<<<<<< HEAD
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
=======
    println!("Setting up tasks");
    setup_metrics(&config);
    tasks.push(service_transaction_stream::<RedisMessenger>(pool, background_task_manager.get_sender(), config.messenger_config.clone()).await);
    statsd_count!("ingester.startup", 1);
>>>>>>> origin/k8s
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

const CHANNEL: &str = "new_item_added";

async fn backfiller(pool: Pool<Postgres>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        println!("Backfiller task running");
        let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool.clone());
        // Connect to db and create PgListener.
        let mut listener = match sqlx::postgres::PgListener::connect_with(&pool.clone()).await {
            Ok(listener) => listener,
            Err(err) => {
                println!("Could not connect to database for PgListener {err}");
                return;
            }
        };
        // Setup listener on channel.
        if let Err(err) = listener.listen(CHANNEL).await {
            println!("Error listening to channel on backfill_items table {err}");
            return;
        }

        loop {
            match get_trees_to_backfill(&db).await {
                Ok(trees) => {
                    if trees.force_chk_trees.len() == 0 && trees.multi_row_trees.len() == 0 {
                        // If there are no trees to backfill, wait for a notification on the channel.
                        let _notification = listener.recv().await.unwrap();
                    } else {
                        // Trees with the `force_chk` flag must be backfilled from seq num 1.
                        println!(
                            "New trees to backfill from seq num 1: {}",
                            trees.force_chk_trees.len()
                        );
                        for tree in trees.force_chk_trees.iter() {
                            let tree_str = tree.tree.encode_hex::<String>();
                            println!("Backfilling tree: {tree_str}");
                            if let Err(err) = backfill_tree_from_seq_1(&db, &tree.tree).await {
                                println!(
                                    "Failed to fetch and plug gaps for {tree_str}, error: {err}"
                                );
                            } else {
                                if let Err(err) = clear_force_chk_flag(&db, &tree.tree).await {
                                    println!("Error clearing force_chk flag: {err}");
                                }
                            }
                        }

                        // Trees with multiple rows must be checked for the range of seq nums in
                        // the `backfill_items` table.
                        println!(
                            "New trees to backfill by detecting gaps: {}",
                            trees.multi_row_trees.len()
                        );
                        for tree in trees.multi_row_trees.iter() {
                            let tree_str = tree.tree.encode_hex::<String>();
                            println!("Backfilling tree: {tree_str}");
                            match fetch_and_plug_gaps(&db, &tree.tree).await {
                                Ok(max_seq) => {
                                    // Only delete extra tree rows if fetching and plugging gaps worked.
                                    if let Err(err) =
                                        delete_extra_tree_rows(&db, &tree.tree, max_seq).await
                                    {
                                        println!("Error deleting rows: {err}");
                                    }
                                }
                                Err(err) => {
                                    println!(
                                        "Failed to fetch and plug gaps for {tree_str}, error: {err}"
                                    );
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    // Print error but keep trying.
                    println!("Could not get trees from db: {err}");
                }
            }
        }
    })
}

#[derive(Debug, FromQueryResult)]
struct UniqueTree {
    tree: Vec<u8>,
}

struct BackfillTrees {
    force_chk_trees: Vec<UniqueTree>,
    multi_row_trees: Vec<UniqueTree>,
}

async fn get_trees_to_backfill(db: &DatabaseConnection) -> Result<BackfillTrees, DbErr> {
    // Get trees with the `force_chk` flag set.
    let force_chk_trees = UniqueTree::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT DISTINCT backfill_items.tree FROM backfill_items WHERE backfill_items.force_chk = TRUE"#,
        vec![],
    ))
    .all(db)
    .await?;

    // Get trees with multiple rows from `backfill_items` table.
    let multi_row_trees = UniqueTree::find_by_statement(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"SELECT backfill_items.tree FROM backfill_items GROUP BY backfill_items.tree HAVING COUNT(*) > 1"#,
        vec![],
    ))
    .all(db)
    .await?;

    Ok(BackfillTrees {
        force_chk_trees,
        multi_row_trees,
    })
}

async fn backfill_tree_from_seq_1(_db: &DatabaseConnection, _tree: &[u8]) -> Result<(), DbErr> {
    //TODO implement gap filler.
    Ok(())
}

async fn fetch_and_plug_gaps(db: &DatabaseConnection, tree: &[u8]) -> Result<i64, DbErr> {
    //TODO implement gap filler, for now just return the max sequence number.
    let items = get_items_with_max_seq(db, tree).await?;

    if items.len() > 0 {
        Ok(items[0].seq)
    } else {
        Ok(0)
    }
}

async fn get_items_with_max_seq(db: &DatabaseConnection, tree: &[u8]) -> Result<Vec<Model>, DbErr> {
    //TODO Find better, simpler query for this.
    backfill_items::Entity::find()
        .filter(backfill_items::Column::Tree.eq(tree))
        .filter(
            Condition::any().add(
                backfill_items::Column::Seq.in_subquery(
                    Query::select()
                        .expr(backfill_items::Column::Seq.max())
                        .from(backfill_items::Entity)
                        .and_where(backfill_items::Column::Tree.eq(tree))
                        .to_owned(),
                ),
            ),
        )
        .all(db)
        .await
}

async fn clear_force_chk_flag(db: &DatabaseConnection, tree: &[u8]) -> Result<UpdateResult, DbErr> {
    backfill_items::Entity::update_many()
        .col_expr(backfill_items::Column::ForceChk, Expr::value(false))
        .filter(backfill_items::Column::Tree.eq(tree))
        .exec(db)
        .await
}

async fn delete_extra_tree_rows(
    db: &DatabaseConnection,
    tree: &[u8],
    seq: i64,
) -> Result<(), DbErr> {
    // Delete all rows in the `backfill_items` table for a specified tree, except for the row with
    // the user-specified sequence number.  One row for each tree must remain so that gaps can be
    // detected after subsequent inserts.
    backfill_items::Entity::delete_many()
        .filter(backfill_items::Column::Tree.eq(tree))
        .filter(backfill_items::Column::Seq.ne(seq))
        .exec(db)
        .await?;

    // Remove any duplicates that have the user-specified seq number (this should not happen under
    // normal circumstances).
    let items = backfill_items::Entity::find()
        .filter(backfill_items::Column::Tree.eq(tree))
        .filter(backfill_items::Column::Seq.eq(seq))
        .all(db)
        .await?;

    if items.len() > 1 {
        for item in items.iter().skip(1) {
            backfill_items::Entity::delete_by_id(item.id)
                .exec(db)
                .await?;
        }
    }

    Ok(())
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
        statsd_count!("ingester.account_update_seen", 1);
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
        if let Some(si) = transaction.slot_index() {
            let slt_idx = format!("{}-{}", transaction.slot(), si);
            statsd_count!("ingester.transaction_event_seen", 1, "slot-idx" => &slt_idx);
        }
        let seen_at = Utc::now();
        statsd_time!("ingester.bus_ingest_time", (seen_at.timestamp_millis() - transaction.seen_at()) as u64);
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
            let program_id = program.key().unwrap();
            let parser = manager.match_program(program_id);
            let str_program_id = bs58::encode(program_id).into_string();
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
                            slot: transaction.slot(),
                        })
                        .await
                        .map_err(|e| {
                            // Just for logging
                            println!("Error in instruction handling onr program {} {:?}", str_program_id, e);
                            e
                        });
                    let finished_at = Utc::now();
                    statsd_time!("ingester.ix_process_time", (finished_at.timestamp_millis() - transaction.seen_at()) as u64, "program_id" => &str_program_id);
                }
                _ => {
                    println!("Program Handler not found for program id {:?}", program);
                }
            }
        }
    }
}
// Associates logs with the given program ID
