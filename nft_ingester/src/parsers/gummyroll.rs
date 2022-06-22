use {
    crate::{
        error::IngesterError, events::handle_event, utils::filter_events_from_logs,
        InstructionBundle, ProgramHandler, ProgramHandlerConfig,
    },
    async_trait::async_trait,
    digital_asset_types::dao::cl_items,
    gummyroll::state::ChangeLogEvent,
    lazy_static::lazy_static,
    sea_orm::{
        entity::*, query::*, sea_query::OnConflict, DatabaseConnection, DatabaseTransaction,
        DbBackend, SqlxPostgresConnector, TransactionTrait,
    },
    serde::Deserialize,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkeys,
    sqlx::{self, Pool, Postgres},
};

#[derive(Debug, Deserialize)]
pub struct CLRecord {
    node_idx: u32,
    level: u32,
    seq: u32,
    hash: String,
}

pubkeys!(
    GummyRollProgramID,
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
        handle_gummyroll_instruction(&bundle.instruction_logs, &self.storage).await
    }
}

impl GummyRollHandler {
    pub fn new(pool: Pool<Postgres>) -> Self {
        GummyRollHandler {
            id: GummyRollProgramID(),
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

pub async fn save_changelog_events(
    gummy_roll_events: Vec<ChangeLogEvent>,
    txn: &DatabaseTransaction,
) -> Result<(), IngesterError> {
    for change_log_event in gummy_roll_events {
        gummyroll_change_log_event_to_database(change_log_event, txn)
            .await
            .map(|_| ())?
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
        Box::pin(async move { save_changelog_events(events, txn).await })
    })
    .await
    .map_err(|db_err| IngesterError::StorageWriteError(db_err.to_string()))
}

pub async fn gummyroll_change_log_event_to_database(
    change_log_event: ChangeLogEvent,
    txn: &DatabaseTransaction,
) -> Result<(), IngesterError> {
    let mut i: i64 = 0;
    for p in change_log_event.path.into_iter() {
        println!("level {}, node {:?}", i, p.node);
        let tree_id = change_log_event.id.as_ref();
        let item = cl_items::ActiveModel {
            tree: Set(tree_id.to_vec()),
            level: Set(i),
            node_idx: Set(p.index as i64),
            hash: Set(p.node.as_ref().to_vec()),
            seq: Set(change_log_event.seq as i64), // this is bad
            ..Default::default()
        };
        i += 1;
        let mut query = cl_items::Entity::insert(item)
            .on_conflict(
                OnConflict::columns([cl_items::Column::Tree, cl_items::Column::NodeIdx])
                    .update_columns([cl_items::Column::Hash, cl_items::Column::Seq])
                    .to_owned(),
            )
            .build(DbBackend::Postgres);
        query.sql = format!("{} WHERE excluded.seq > cl_items.seq", query.sql);
        txn.execute(query)
            .await
            .map_err(|db_err| IngesterError::StorageWriteError(db_err.to_string()))?;
    }
    Ok(())
    //TODO -> set maximum size of path and break into multiple statements
}
