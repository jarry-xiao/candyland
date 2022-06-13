use {
    gummyroll::state::change_log::ChangeLogEvent,
    serde::Deserialize,
    sqlx::{self, Pool, Postgres},
    std::fs::File,
    async_trait::async_trait,
    lazy_static::lazy_static,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkeys,
    crate::{
        ProgramHandler,
        ProgramHandlerConfig,
        error::IngesterError,
        events::handle_event,
        utils::{filter_events_from_logs, write_assets_to_file},
        InstructionBundle,
    },
};

const SET_CLSQL_ITEM: &str = "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5) ON conflict node_idx DO UPDATE SET hash = EXCLUDED.hash, seq = EXCLUDED.seq";

#[derive(Debug, Deserialize)]
pub struct CLRecord {
    node_idx: u32,
    level: u32,
    seq: u32,
    hash: String,
}

pubkeys!(
    Gummy_Roll_Program_ID,
    "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD"
);

pub struct GummyRollHandler {
    id: Pubkey,
    storage: Pool<Postgres>,
}

#[async_trait]
impl ProgramHandler for GummyRollHandler {
    fn id(&self) -> Pubkey {
        self.id
    }

    fn config(&self) -> &ProgramHandlerConfig {
        lazy_static! {
            static ref CONFIG: ProgramHandlerConfig = ProgramHandlerConfig {
                responds_to_instruction: true,
                responds_to_account: false
            };
        }
        return &CONFIG;
    }

    async fn handle_instruction(&self, bundle: &InstructionBundle) -> Result<(), IngesterError> {
        handle_gummyroll_instruction(
            &bundle.instruction_logs,
            bundle.message_id,
            &self.storage,
        )
            .await
    }
}

impl GummyRollHandler {
    pub fn new(pool: Pool<Postgres>) -> Self {
        GummyRollHandler {
            id: Gummy_Roll_Program_ID(),
            storage: pool,
        }
    }
}

pub fn get_gummy_roll_events(logs: &Vec<&str>) -> Result<Vec<ChangeLogEvent>, IngesterError> {
    let change_log_event_vec = if let Ok(change_log_event_vec) = filter_events_from_logs(logs) {
        change_log_event_vec
    } else {
        println!("Could not find emitted program data");
        return Err(IngesterError::ChangeLogEventMalformed);
    };
    let mut events = vec![];
    // Parse each change log event found in logs
    for event in change_log_event_vec {
        let change_log_event = if let Ok(change_log_event) = handle_event(event) {
            events.push(change_log_event);
        } else {
            println!("\tBad change log event data");
            continue;
        };
    }
    Ok(events)
}

pub async fn handle_gummyroll_instruction(
    logs: &Vec<&str>,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), IngesterError> {
    for change_log_event in get_gummy_roll_events(logs)? {
        // Put change log event into database.
        change_log_db_txn(change_log_event, pid, pool).await;
    }
    Ok(())
}

async fn change_log_db_txn(
    change_log_event: ChangeLogEvent,
    pid: i64,
    pool: &Pool<Postgres>,
) {
    println!("\tCL tree {:?}", change_log_event.id);
    let txnb = pool.begin().await;
    match txnb {
        Ok(txn) => {
            gummyroll_change_log_event_to_database(
                change_log_event,
                pid,
                pool,
            ).await;
            match txn.commit().await {
                Ok(_r) => {
                    println!("Saved CL");
                }
                Err(e) => {
                    eprintln!("{}", e.to_string())
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e.to_string())
        }
    }
}

pub async fn gummyroll_change_log_event_to_database(
    change_log_event: ChangeLogEvent,
    pid: i64,
    pool: &Pool<Postgres>,
) {
    let mut i: i64 = 0;
    for p in change_log_event.path.into_iter() {
        println!("level {}, node {:?}", i, p.node.inner);
        let tree_id = change_log_event.id.as_ref();
        let f = sqlx::query(SET_CLSQL_ITEM)
            .bind(&tree_id)
            .bind(&pid + i)
            .bind(&i)
            .bind(&p.node.inner.as_ref())
            .bind(&(p.index as i64))
            .execute(pool)
            .await;
        if f.is_err() {
            println!("Error {:?}", f.err().unwrap());
        }
        i += 1;
    }
}
