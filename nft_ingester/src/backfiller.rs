//! Backfiller that fills gaps in trees by detecting gaps in sequence numbers
//! in the `backfill_items` table.  Inspired by backfiller.ts/backfill.ts.

use {
    crate::{
        error::IngesterError, parsers::*, IngesterConfig, DATABASE_LISTENER_CHANNEL_KEY,
        RPC_URL_KEY,
    },
    digital_asset_types::dao::backfill_items,
    flatbuffers::FlatBufferBuilder,
    messenger::{Messenger, TRANSACTION_STREAM},
    plerkle_serialization::transaction_info_generated::transaction_info::{
        self, TransactionInfo, TransactionInfoArgs,
    },
    sea_orm::{
        entity::*,
        query::*,
        sea_query::{Expr, Query},
        DatabaseConnection, DbBackend, DbErr, FromQueryResult, SqlxPostgresConnector,
    },
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey},
    solana_transaction_status::{UiInstruction::Compiled, UiRawMessage, UiTransactionStatusMeta},
    sqlx::{self, postgres::PgListener, Pool, Postgres},
    std::str::FromStr,
    tokio::time::{sleep, Duration},
};

// Constants used for varying delays when failures occur.
const INITIAL_FAILURE_DELAY: u64 = 100;
const MAX_FAILURE_DELAY_MS: u64 = 10_000;

// Account key used to determine if transaction is a simple vote.
const VOTE: &str = "Vote111111111111111111111111111111111111111";

/// Main public entry point for backfiller task.
pub async fn backfiller<T: Messenger>(
    pool: Pool<Postgres>,
    config: IngesterConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        println!("Backfiller task running");

        if let Ok(mut backfiller) = Backfiller::<T>::new(pool, config).await {
            backfiller.run().await;
        }
    })
}

/// Struct used when querying for unique trees.
#[derive(Debug, FromQueryResult)]
struct UniqueTree {
    tree: Vec<u8>,
}

/// Struct used when storing which trees to backfill.
struct BackfillTrees {
    force_chk_trees: Vec<UniqueTree>,
    multi_row_trees: Vec<UniqueTree>,
}

/// Struct used when querying the max sequence number of a tree.
#[derive(Debug, FromQueryResult, Clone)]
struct MaxSeqItem {
    max: i64,
}

/// Struct used when querying for items to backfill.
#[derive(Debug, FromQueryResult, Clone)]
struct SimpleBackfillItem {
    seq: i64,
    slot: i64,
}

/// Struct used to store sequence number gap info for a given tree.
struct GapInfo {
    prev: SimpleBackfillItem,
    curr: SimpleBackfillItem,
}

impl GapInfo {
    fn new(prev: SimpleBackfillItem, curr: SimpleBackfillItem) -> Self {
        Self { prev, curr }
    }
}

/// Main struct used for backfiller task.
struct Backfiller<T: Messenger> {
    db: DatabaseConnection,
    listener: PgListener,
    rpc_client: RpcClient,
    messenger: T,
    failure_delay: u64,
}

impl<T: Messenger> Backfiller<T> {
    /// Create a new `Backfiller` struct.
    async fn new(pool: Pool<Postgres>, config: IngesterConfig) -> Result<Self, ()> {
        // Create Sea ORM database connection used later for queries.
        let db = SqlxPostgresConnector::from_sqlx_postgres_pool(pool.clone());

        // Connect to database using sqlx and create PgListener.
        let mut listener = match sqlx::postgres::PgListener::connect_with(&pool.clone()).await {
            Ok(listener) => listener,
            Err(err) => {
                println!("Could not connect to database for PgListener {err}");
                return Err(());
            }
        };

        // Get database listener channel.
        let channel = config
            .database_config
            .get(&*DATABASE_LISTENER_CHANNEL_KEY)
            .and_then(|u| u.clone().into_string())
            .ok_or(IngesterError::ConfigurationError {
                msg: format!(
                    "Database listener channel missing: {}",
                    DATABASE_LISTENER_CHANNEL_KEY
                ),
            })
            .unwrap();

        // Setup listener on channel.
        if let Err(err) = listener.listen(&channel).await {
            println!("Error listening to channel on backfill_items table {err}");
            return Err(());
        }

        // Get RPC URL.
        let rpc_url = config
            .rpc_config
            .get(&*RPC_URL_KEY)
            .and_then(|u| u.clone().into_string())
            .ok_or(IngesterError::ConfigurationError {
                msg: format!("RPC URL missing: {}", RPC_URL_KEY),
            })
            .unwrap();

        // Instantiate RPC client.
        let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::processed());

        // Instantiate messenger.
        let mut messenger = T::new(config.messenger_config).await.unwrap();
        messenger.add_stream(TRANSACTION_STREAM).await;
        messenger.set_buffer_size(TRANSACTION_STREAM, 5000).await;

        Ok(Self {
            db,
            listener,
            rpc_client,
            messenger,
            failure_delay: INITIAL_FAILURE_DELAY,
        })
    }

    /// Run the backfiller task.
    async fn run(&mut self) {
        loop {
            match self.get_trees_to_backfill().await {
                Ok(trees) => {
                    if trees.force_chk_trees.len() == 0 && trees.multi_row_trees.len() == 0 {
                        // If there are no trees to backfill, wait for a notification on the channel.
                        let _notification = self.listener.recv().await.unwrap();
                    } else {
                        // First just check if we can talk to an RPC provider.
                        match self.rpc_client.get_version().await {
                            Ok(version) => println!("RPC client version {version}"),
                            Err(err) => {
                                println!("RPC client error {err}");
                                self.sleep_and_increase_delay().await;
                                continue;
                            }
                        }

                        // Trees with the `force_chk` flag must be backfilled from seq num 1.
                        println!(
                            "New trees to backfill from seq num 1: {}",
                            trees.force_chk_trees.len()
                        );
                        for tree in trees.force_chk_trees.iter() {
                            let tree_str = bs58::encode(&tree.tree).into_string();
                            println!("Backfilling tree: {tree_str}");

                            match self.backfill_tree_from_seq_1(&tree.tree).await {
                                Ok(opt_max_seq) => {
                                    if let Some(_max_seq) = opt_max_seq {
                                        // Debug.
                                        println!(
                                            "Successfully backfilled tree from seq 1: {tree_str}"
                                        );

                                        // Only delete extra tree rows if fetching and plugging gaps worked.
                                        if let Err(err) =
                                            self.clear_force_chk_flag(&tree.tree).await
                                        {
                                            println!("Error clearing force_chk flag: {err}");
                                            self.sleep_and_increase_delay().await;
                                        } else {
                                            // Debug.
                                            println!("Successfully cleared force_chk flag");
                                            self.reset_delay();
                                        }
                                    } else {
                                        // Debug.
                                        println!("Unexpected error, tree was in list, but no rows found for {tree_str}");
                                    }
                                }
                                Err(err) => {
                                    println!("Failed to fetch and plug gaps for {tree_str}");
                                    println!("{err}");
                                    self.sleep_and_increase_delay().await;
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
                            let tree_str = bs58::encode(&tree.tree).into_string();
                            println!("Backfilling tree: {tree_str}");

                            match self.fetch_and_plug_gaps(&tree.tree).await {
                                Ok(opt_max_seq) => {
                                    if let Some(max_seq) = opt_max_seq {
                                        // Debug.
                                        println!(
                                            "Successfully fetched and plug gaps for {tree_str}"
                                        );

                                        // Only delete extra tree rows if fetching and plugging gaps worked.
                                        if let Err(err) =
                                            self.delete_extra_tree_rows(&tree.tree, max_seq).await
                                        {
                                            println!("Error deleting rows: {err}");
                                            self.sleep_and_increase_delay().await;
                                        } else {
                                            // Debug.
                                            println!("Successfully deleted rows up to {max_seq}");
                                            self.reset_delay()
                                        }
                                    } else {
                                        // Debug.
                                        println!("Unexpected error, tree was in list, but no rows found for {tree_str}");
                                    }
                                }
                                Err(err) => {
                                    println!("Failed to fetch and plug gaps for {tree_str}");
                                    println!("{err}");
                                    self.sleep_and_increase_delay().await;
                                }
                            }
                        }
                    }
                }
                Err(err) => {
                    // Print error but keep trying.
                    println!("Could not get trees from db: {err}");
                    self.sleep_and_increase_delay().await;
                }
            }
        }
    }

    async fn sleep_and_increase_delay(&mut self) {
        sleep(Duration::from_millis(self.failure_delay)).await;

        // Increase failure delay up to `MAX_FAILURE_DELAY_MS`.
        self.failure_delay = self.failure_delay.saturating_mul(2);
        self.failure_delay = if self.failure_delay > MAX_FAILURE_DELAY_MS {
            MAX_FAILURE_DELAY_MS
        } else {
            self.failure_delay
        };
    }

    fn reset_delay(&mut self) {
        self.failure_delay = INITIAL_FAILURE_DELAY;
    }

    async fn get_trees_to_backfill(&self) -> Result<BackfillTrees, DbErr> {
        // Get trees with the `force_chk` flag set.
        let force_chk_trees = UniqueTree::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT DISTINCT backfill_items.tree FROM backfill_items WHERE backfill_items.force_chk = TRUE"#,
            vec![],
        ))
        .all(&self.db)
        .await?;

        // Get trees with multiple rows from `backfill_items` table.
        let multi_row_trees = UniqueTree::find_by_statement(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"SELECT backfill_items.tree FROM backfill_items GROUP BY backfill_items.tree HAVING COUNT(*) > 1"#,
            vec![],
        ))
        .all(&self.db)
        .await?;

        Ok(BackfillTrees {
            force_chk_trees,
            multi_row_trees,
        })
    }

    async fn backfill_tree_from_seq_1(&self, tree: &[u8]) -> Result<Option<i64>, DbErr> {
        //TODO implement gap filler that gap fills from sequence number 1.
        //For now just return the max sequence number.
        self.get_max_seq(tree).await
    }

    async fn get_max_seq(&self, tree: &[u8]) -> Result<Option<i64>, DbErr> {
        //TODO Find better, simpler query for this.
        let rows = backfill_items::Entity::find()
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
            .all(&self.db)
            .await?;

        Ok(rows.last().map(|row| row.seq))
    }

    async fn clear_force_chk_flag(&self, tree: &[u8]) -> Result<UpdateResult, DbErr> {
        backfill_items::Entity::update_many()
            .col_expr(backfill_items::Column::ForceChk, Expr::value(false))
            .filter(backfill_items::Column::Tree.eq(tree))
            .exec(&self.db)
            .await
    }

    // Similar to `fetchAndPlugGaps()` in `backfiller.ts`.
    async fn fetch_and_plug_gaps(&mut self, tree: &[u8]) -> Result<Option<i64>, DbErr> {
        let (opt_max_seq, gaps) = self.get_missing_data(tree).await?;

        // Similar to `plugGapsBatched()` in `backfiller.ts` (although not batched).
        for gap in gaps.iter() {
            // Similar to `plugGaps()` in `backfiller.ts`.
            let _result = self.plug_gap(&gap, tree).await?;
        }

        Ok(opt_max_seq)
    }

    // Similar to `getMissingData()` in `db.ts`.
    async fn get_missing_data(&self, tree: &[u8]) -> Result<(Option<i64>, Vec<GapInfo>), DbErr> {
        // Get the maximum sequence number that has been backfilled, and use
        // that for the starting sequence number for backfilling.
        let mut query = backfill_items::Entity::find()
            .select_only()
            .column(backfill_items::Column::Seq)
            .filter(backfill_items::Column::Backfilled.eq(true))
            .filter(backfill_items::Column::Tree.eq(tree))
            .group_by(backfill_items::Column::Tree)
            .build(DbBackend::Postgres);

        query.sql = query.sql.replace(
            "SELECT \"backfill_items\".\"seq\"",
            "SELECT MAX (\"backfill_items\".\"seq\")",
        );

        // Debug.
        //println!("{}", query.to_string());

        let start_seq_vec = MaxSeqItem::find_by_statement(query).all(&self.db).await?;
        let start_seq = if start_seq_vec.len() > 0 {
            start_seq_vec[0].max
        } else {
            0
        };

        // Debug.
        //println!("MAX: {}", start_seq);

        // Get all rows for the tree that have not yet been backfilled.
        let mut query = backfill_items::Entity::find()
            .select_only()
            .column(backfill_items::Column::Seq)
            .column(backfill_items::Column::Slot)
            .filter(backfill_items::Column::Seq.gte(start_seq))
            .filter(backfill_items::Column::Tree.eq(tree))
            .order_by_asc(backfill_items::Column::Seq)
            .build(DbBackend::Postgres);

        query.sql = query.sql.replace("SELECT", "SELECT DISTINCT");
        let rows = SimpleBackfillItem::find_by_statement(query)
            .all(&self.db)
            .await?;
        let mut gaps = vec![];

        // Look at each pair of trees looking for a gap in sequence number.
        for (prev, curr) in rows.iter().zip(rows.iter().skip(1)) {
            if curr.seq == prev.seq {
                let message = format!(
                    "Error in DB, identical sequence numbers with different slots: {}, {}",
                    prev.slot, curr.slot
                );
                println!("{}", message);
                return Err(DbErr::Custom(message));
            } else if curr.seq - prev.seq > 1 {
                gaps.push(GapInfo::new(prev.clone(), curr.clone()));
            }
        }

        // Get the max sequence number if any rows were returned from the query.
        let opt_max_seq = rows.last().map(|row| row.seq);

        Ok((opt_max_seq, gaps))
    }

    // Similar to `plugGaps()` in `backfiller.ts`.
    async fn plug_gap(&mut self, gap: &GapInfo, tree: &[u8]) -> Result<(), DbErr> {
        // TODO: This needs to make sure all slots are available otherwise it will partially
        // fail and redo the whole backfill process.  So for now checking the max block before
        // looping as a quick workaround.
        let _ = self
            .rpc_client
            .get_block(gap.curr.slot as u64)
            .await
            .map_err(|e| {
                DbErr::Custom(format!("Blocks needed for backfilling not finalized: {e}"))
            })?;

        for slot in gap.prev.slot..gap.curr.slot {
            let block_data = self
                .rpc_client
                .get_block(slot as u64)
                .await
                .map_err(|e| DbErr::Custom(e.to_string()))?;

            // Debug.
            println!("num txs: {}", block_data.transactions.len());
            for tx in block_data.transactions {
                // Debug.
                //println!("HERE IS THE TX");
                //println!("{:#?}", tx);

                // See if transaction has an error.
                let meta = if let Some(meta) = &tx.meta {
                    if let Some(err) = &meta.err {
                        println!("Transaction has error: {err}");
                        continue;
                    }
                    meta
                } else {
                    println!("Unexpected, EncodedTransactionWithStatusMeta struct has no metadata");
                    continue;
                };

                // Get `UiTransaction` out of `EncodedTransactionWithStatusMeta`.
                let ui_transaction = match tx.transaction {
                    solana_transaction_status::EncodedTransaction::Json(ref ui_transaction) => {
                        ui_transaction
                    }
                    _ => {
                        return Err(DbErr::Custom(
                            "Unsupported format for EncodedTransaction".to_string(),
                        ));
                    }
                };

                // See if transaction is a vote.
                let ui_raw_message = match &ui_transaction.message {
                    solana_transaction_status::UiMessage::Raw(ui_raw_message) => {
                        if ui_raw_message.account_keys.iter().any(|key| key == VOTE) {
                            // Debug.
                            println!("Skipping vote transaction");
                            continue;
                        } else {
                            ui_raw_message
                        }
                    }
                    _ => {
                        return Err(DbErr::Custom(
                            "Unsupported format for UiMessage".to_string(),
                        ));
                    }
                };

                // Filter out transactions that don't have to do with the tree we are interested in or
                // the Bubblegum program.
                let tree = bs58::encode(tree).into_string();
                let bubblegum = BubblegumProgramID().to_string();
                if ui_raw_message
                    .account_keys
                    .iter()
                    .all(|pk| *pk != tree && *pk != bubblegum)
                {
                    // Debug.
                    println!("This transaction is being skipped\n\n");
                    continue;
                }

                // Debug.
                println!("Serializing transaction");
                // Serialize data.
                let builder = FlatBufferBuilder::new();
                let builder = serialize_transaction(
                    builder,
                    &meta,
                    &ui_raw_message,
                    slot.try_into().unwrap(),
                );

                // Debug.
                println!("Putting data into Redis");
                // Put data into Redis.
                let _ = self
                    .messenger
                    .send(TRANSACTION_STREAM, builder.finished_data())
                    .await
                    .map_err(|e| DbErr::Custom(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn delete_extra_tree_rows(&self, tree: &[u8], seq: i64) -> Result<(), DbErr> {
        // Debug.
        let test_items = backfill_items::Entity::find()
            .filter(backfill_items::Column::Tree.eq(tree))
            .all(&self.db)
            .await?;
        println!("Count of items before delete: {}", test_items.len());
        for item in test_items {
            println!("Seq ID {}", item.seq);
        }

        // Delete all rows in the `backfill_items` table for a specified tree, except for the row with
        // the user-specified sequence number.  One row for each tree must remain so that gaps can be
        // detected after subsequent inserts.
        backfill_items::Entity::delete_many()
            .filter(backfill_items::Column::Tree.eq(tree))
            .filter(backfill_items::Column::Seq.ne(seq))
            .exec(&self.db)
            .await?;

        // Remove any duplicates that have the user-specified seq number (this should not happen under
        // normal circumstances).
        let items = backfill_items::Entity::find()
            .filter(backfill_items::Column::Tree.eq(tree))
            .filter(backfill_items::Column::Seq.eq(seq))
            .all(&self.db)
            .await?;

        if items.len() > 1 {
            for item in items.iter().skip(1) {
                backfill_items::Entity::delete_by_id(item.id)
                    .exec(&self.db)
                    .await?;
            }
        }

        // Mark remaining row as backfilled so future backfilling can start above this sequence number.
        backfill_items::Entity::update_many()
            .col_expr(backfill_items::Column::Backfilled, Expr::value(true))
            .filter(backfill_items::Column::Tree.eq(tree))
            .exec(&self.db)
            .await?;

        // Debug.
        let test_items = backfill_items::Entity::find()
            .filter(backfill_items::Column::Tree.eq(tree))
            .all(&self.db)
            .await?;
        println!("Count of items after delete: {}", test_items.len());
        for item in test_items {
            println!("Seq ID {}", item.seq);
        }

        Ok(())
    }
}

fn serialize_transaction<'a>(
    mut builder: FlatBufferBuilder<'a>,
    meta: &UiTransactionStatusMeta,
    ui_raw_message: &UiRawMessage,
    slot: u64,
) -> FlatBufferBuilder<'a> {
    // Serialize account keys.
    let account_keys = &ui_raw_message.account_keys;
    let account_keys_len = account_keys.len();

    let account_keys = if account_keys_len > 0 {
        let mut account_keys_fb_vec = Vec::with_capacity(account_keys_len);
        for key in account_keys.iter() {
            // TODO deal with this failure.
            let key = Pubkey::from_str(key).unwrap();

            let key = builder.create_vector(&key.to_bytes());
            let pubkey = transaction_info::Pubkey::create(
                &mut builder,
                &transaction_info::PubkeyArgs { key: Some(key) },
            );
            account_keys_fb_vec.push(pubkey);
        }
        Some(builder.create_vector(&account_keys_fb_vec))
    } else {
        None
    };

    // Serialize log messages.
    let log_messages = if let Some(log_messages) = meta.log_messages.as_ref() {
        let mut log_messages_fb_vec = Vec::with_capacity(log_messages.len());
        for message in log_messages {
            log_messages_fb_vec.push(builder.create_string(&message));
        }
        Some(builder.create_vector(&log_messages_fb_vec))
    } else {
        None
    };

    // Serialize inner instructions.
    let inner_instructions = if let Some(inner_instructions_vec) = meta.inner_instructions.as_ref()
    {
        let mut overall_fb_vec = Vec::with_capacity(inner_instructions_vec.len());
        for inner_instructions in inner_instructions_vec.iter() {
            let index = inner_instructions.index;
            let mut instructions_fb_vec = Vec::with_capacity(inner_instructions.instructions.len());
            for ui_instruction in inner_instructions.instructions.iter() {
                if let Compiled(ui_compiled_instruction) = ui_instruction {
                    let program_id_index = ui_compiled_instruction.program_id_index;
                    let accounts = Some(builder.create_vector(&ui_compiled_instruction.accounts));

                    // TODO deal with this failure.
                    let data = bs58::decode(&ui_compiled_instruction.data)
                        .into_vec()
                        .unwrap();

                    let data = Some(builder.create_vector(&data));
                    instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                        &mut builder,
                        &transaction_info::CompiledInstructionArgs {
                            program_id_index,
                            accounts,
                            data,
                        },
                    ));
                }
            }

            let instructions = Some(builder.create_vector(&instructions_fb_vec));
            overall_fb_vec.push(transaction_info::InnerInstructions::create(
                &mut builder,
                &transaction_info::InnerInstructionsArgs {
                    index,
                    instructions,
                },
            ))
        }

        Some(builder.create_vector(&overall_fb_vec))
    } else {
        None
    };

    // Serialize outer instructions.
    let outer_instructions = &ui_raw_message.instructions;
    let outer_instructions = if outer_instructions.len() > 0 {
        let mut instructions_fb_vec = Vec::with_capacity(outer_instructions.len());
        for ui_compiled_instruction in outer_instructions.iter() {
            let program_id_index = ui_compiled_instruction.program_id_index;
            let accounts = Some(builder.create_vector(&ui_compiled_instruction.accounts));

            // TODO deal with this failure.
            let data = bs58::decode(&ui_compiled_instruction.data)
                .into_vec()
                .unwrap();

            let data = Some(builder.create_vector(&data));
            instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                &mut builder,
                &transaction_info::CompiledInstructionArgs {
                    program_id_index,
                    accounts,
                    data,
                },
            ));
        }
        Some(builder.create_vector(&instructions_fb_vec))
    } else {
        None
    };

    // Serialize everything into Transaction Info table.
    let transaction_info = TransactionInfo::create(
        &mut builder,
        &TransactionInfoArgs {
            is_vote: false,
            account_keys,
            log_messages,
            inner_instructions,
            outer_instructions,
            slot,
        },
    );

    // Finalize buffer and return to caller.
    builder.finish(transaction_info, None);
    builder
}
