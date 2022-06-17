use {
    sea_orm::{
        DbErr,
        entity::*,
        query::*,
        DatabaseConnection, DatabaseTransaction,
        JsonValue, SqlxPostgresConnector, TransactionTrait,
    },
    gummyroll::state::change_log::ChangeLogEvent,
    serde::Deserialize,
    sqlx::{self, Pool, Postgres},
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
use digital_asset_types::dao::cl_items;

const SET_CLSQL_ITEM: &str = r#"
INSERT INTO cl_items (tree, seq, level, hash, node_idx)
VALUES ($1,$2,$3,$4,$5) ON CONFLICT (tree, node_idx)
DO UPDATE SET hash = EXCLUDED.hash, seq = EXCLUDED.seq
"#;

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
    storage: DatabaseConnection,
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
            &self.storage,
        )
            .await
    }
}

impl GummyRollHandler {
    pub fn new(pool: Pool<Postgres>) -> Self {
        GummyRollHandler {
            id: Gummy_Roll_Program_ID(),
            storage: SqlxPostgresConnector::from_sqlx_postgres_pool(pool),
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
         if let Ok(change_log_event) = handle_event(event) {
            events.push(change_log_event);
        } else {
            continue;
        };
    }
    Ok(events)
}

pub async fn save_changelog_events(gummy_roll_events: Vec<ChangeLogEvent>, txn: &DatabaseTransaction) -> Result<(), IngesterError> {
    for change_log_event in gummy_roll_events {
        gummyroll_change_log_event_to_database(
            change_log_event,
            txn,
        ).await.map(|_| ())?
    }
    Ok(())
}

pub async fn handle_gummyroll_instruction(
    logs: &Vec<&str>,
    db: &DatabaseConnection,
) -> Result<(), IngesterError> {
    // map to owned vec to avoid static lifetime issues, instead of moving logs into Box
    let events = get_gummy_roll_events(logs)?;
    db.transaction::<_, _, IngesterError>(|txn| {
        Box::pin(async move {
            save_changelog_events(events, txn).await
        })
    }).await
        .map_err(|db_err| {
            IngesterError::StorageWriteError(db_err.to_string())
        })
}

pub async fn gummyroll_change_log_event_to_database(
    change_log_event: ChangeLogEvent,
    txn: &DatabaseTransaction,
) -> Result<InsertResult<cl_items::ActiveModel>, IngesterError> {
    let mut i: i64 = 0;
    let mut items = Vec::with_capacity(change_log_event.path.len());
    for p in change_log_event.path.into_iter() {
        println!("level {}, node {:?}", i, p.node.inner);
        let tree_id = change_log_event.id.as_ref();
        let item = cl_items::ActiveModel {
            tree: Set(tree_id.to_vec()),
            level: Set(i),
            node_idx: Set(p.index as i64),
            hash: Set(p.node.inner.as_ref().to_vec()),
            seq: Set(change_log_event.seq as i64), // this is bad
            ..Default::default()
        };
        items.push(item);
        i += 1;
    }
    cl_items::Entity::insert_many(items).exec(txn).await.map_err(|db_err| {
        IngesterError::StorageWriteError(db_err.to_string())
    })
}
