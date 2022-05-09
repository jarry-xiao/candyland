use {
    anchor_client::anchor_lang::AnchorDeserialize,
    flatbuffers::{ForwardsUOffset, Vector},
    gummyroll::state::change_log::ChangeLogEvent,
    lazy_static::lazy_static,
    messenger::{ACCOUNT_STREAM, BLOCK_STREAM, DATA_KEY, SLOT_STREAM, TRANSACTION_STREAM},
    nft_api_lib::events::handle_event,
    plerkle_serialization::transaction_info_generated::transaction_info::{
        self, root_as_transaction_info, TransactionInfo,
    },
    redis::{
        streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply},
        Commands, Value,
    },
    regex::Regex,
    solana_sdk::keccak,
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
        gummy_roll_crud,
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
    );
    pubkeys!(gummy_roll, "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");
    pubkeys!(token, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    pubkeys!(a_token, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}

#[derive(Default)]
struct AppEvent {
    op: String,
    message: String,
    leaf: String,
    owner: String,
    new_owner: Option<String>,
    tree_id: String,
    authority: String,
}

const SET_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (msg)
                            DO UPDATE SET leaf = excluded.leaf, owner = excluded.owner, tree_id = excluded.tree_id, revision = excluded.revision"#;
const SET_OWNERSHIP_APPSQL: &str = r#"INSERT INTO app_specific_ownership (tree_id, authority) VALUES ($1,$2) ON CONFLICT (tree_id)
                            DO UPDATE SET authority = excluded.authority"#;
const GET_APPSQL: &str = "SELECT revision FROM app_specific WHERE msg = $1 AND tree_id = $2";
const DEL_APPSQL: &str = "DELETE FROM app_specific WHERE leaf = $1 AND tree_id = $2";
const SET_CLSQL_ITEM: &str =
    "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5)";

#[derive(sqlx::FromRow, Clone, Debug)]
struct AppSpecificRev {
    revision: i64,
}

#[tokio::main]
async fn main() {
    let client = redis::Client::open("redis://redis/").unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://solana:solana@db/solana")
        .await
        .unwrap();
    let ids = [">", ">", ">", ">"];
    let mut conn = client.get_connection().unwrap();
    let streams = [
        ACCOUNT_STREAM,
        SLOT_STREAM,
        TRANSACTION_STREAM,
        BLOCK_STREAM,
    ];
    const GROUP_NAME: &str = "ingester";

    for key in &streams {
        let created: Result<(), _> = conn.xgroup_create_mkstream(*key, GROUP_NAME, "$");
        if let Err(e) = created {
            println!("Group already exists: {:?}", e)
        }
    }

    let opts = StreamReadOptions::default()
        .block(1000)
        .count(100000)
        .group(GROUP_NAME, "lelelelle");

    loop {
        let srr: StreamReadReply = conn.xread_options(&streams, &ids, &opts).unwrap();

        for StreamKey { key, ids } in srr.keys {
            if key == ACCOUNT_STREAM {
                // Do nothing for now.
            } else if key == SLOT_STREAM {
                // Do nothing for now.
            } else if key == TRANSACTION_STREAM {
                println!("{}", key);
                handle_transaction(&ids, &pool).await;
            } else if key == BLOCK_STREAM {
                // Do nothing for now.
            }
        }
    }
}

pub async fn handle_transaction(ids: &Vec<StreamId>, pool: &Pool<Postgres>) {
    for StreamId { id, map } in ids {
        let pid = id.replace("-", "").parse::<i64>().unwrap();

        // Get data from map.
        let data = if let Some(data) = map.get(DATA_KEY) {
            data
        } else {
            println!("No Data was stored in Redis for ID {id}");
            continue;
        };
        let bytes = match data {
            Value::Data(bytes) => bytes,
            _ => {
                println!("Redis data for ID {id} in wrong format");
                continue;
            }
        };

        // Get root of transaction info flatbuffers object.
        let transaction = match root_as_transaction_info(&bytes) {
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

        // Handle log message parsing.
        if keys
            .iter()
            .any(|pubkey| pubkey.key().unwrap() == program_ids::gummy_roll().to_bytes())
        {
            println!("Found GM CL event");
            // Get vector of change log events.
            let change_log_event_vec = if let Ok(change_log_event_vec) =
                handle_change_log_event(transaction.log_messages())
            {
                change_log_event_vec
            } else {
                println!("Could not handle change log event vector");
                continue;
            };

            // Get each change log event in the vector.
            for event in change_log_event_vec {
                let change_log_event = if let Ok(change_log_event) = handle_event(event) {
                    change_log_event
                } else {
                    println!("\tBad change log event data");
                    continue;
                };

                // Put change log event into database.
                change_log_event_to_database(change_log_event, pid, pool).await;
            }
        }

        // Handle instruction parsing.
        let instructions = order_instructions(&transaction);
        for program_instruction in instructions {
            match program_instruction {
                (program, instruction) if program == program_ids::gummy_roll_crud() => {
                    let mut app_event = AppEvent::default();
                    match gummyroll_crud::get_instruction_type(&instruction.data) {
                        gummyroll_crud::InstructionName::CreateTree => {
                            // Get tree ID.
                            let tree_id =
                                pubkey_from_fb_table(&keys, instruction.accounts[3] as usize);

                            // Get authority.
                            let auth =
                                pubkey_from_fb_table(&keys, instruction.accounts[0] as usize);

                            // Populate app event.
                            app_event.op = String::from("create");
                            app_event.tree_id = tree_id;
                            app_event.authority = auth;
                        }
                        gummyroll_crud::InstructionName::Add => {
                            // Get data.
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let add: gummyroll_crud::instruction::Add =
                                gummyroll_crud::instruction::Add::deserialize(data_buf).unwrap();

                            // Get tree ID.
                            let tree_id =
                                pubkey_from_fb_table(&keys, instruction.accounts[3] as usize);

                            // Get owner from index 0.
                            let owner =
                                pubkey_from_fb_table(&keys, instruction.accounts[0] as usize);

                            // Get message and leaf.
                            let hex_message = hex::encode(&add.message);
                            let leaf = keccak::hashv(&[&owner.as_bytes(), add.message.as_slice()]);

                            // Populate app event.
                            app_event.op = String::from("add");
                            app_event.tree_id = tree_id;
                            app_event.leaf = leaf.to_string();
                            app_event.message = hex_message;
                            app_event.owner = owner;
                        }
                        gummyroll_crud::InstructionName::Transfer => {
                            // Get data.
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let add: gummyroll_crud::instruction::Transfer =
                                gummyroll_crud::instruction::Transfer::deserialize(data_buf)
                                    .unwrap();

                            // Get tree ID.
                            let tree_id =
                                pubkey_from_fb_table(&keys, instruction.accounts[3] as usize);

                            // Get owner from index 4.
                            let owner =
                                pubkey_from_fb_table(&keys, instruction.accounts[4] as usize);

                            // Get new owner from index 5.
                            let new_owner =
                                pubkey_from_fb_table(&keys, instruction.accounts[5] as usize);

                            // Get message and leaf.
                            let hex_message = hex::encode(&add.message);
                            let leaf =
                                keccak::hashv(&[&new_owner.as_bytes(), add.message.as_slice()]);

                            // Populate app event.
                            app_event.op = String::from("tran");
                            app_event.tree_id = tree_id;
                            app_event.leaf = leaf.to_string();
                            app_event.message = hex_message;
                            app_event.owner = owner;
                            app_event.new_owner = Some(new_owner);
                        }
                        gummyroll_crud::InstructionName::Remove => {
                            // Get data.
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let remove: gummyroll_crud::instruction::Remove =
                                gummyroll_crud::instruction::Remove::deserialize(data_buf).unwrap();

                            // Get tree ID.
                            let tree_id =
                                pubkey_from_fb_table(&keys, instruction.accounts[3] as usize);

                            // Get owner from index 0.
                            let owner =
                                pubkey_from_fb_table(&keys, instruction.accounts[0] as usize);

                            // Get leaf.
                            let leaf = bs58::encode(&remove.leaf_hash).into_string();

                            // Populate app event.
                            app_event.op = String::from("rm");
                            app_event.tree_id = tree_id;
                            app_event.leaf = leaf.to_string();
                            app_event.message = "".to_string();
                            app_event.owner = owner;
                        }
                        _ => {}
                    }
                    // If we populated an app event, write it to the database.
                    if app_event.op.len() > 0 {
                        let _result = app_event_to_database(&app_event, pid, pool).await;
                    }
                }
                _ => {}
            }
        }
    }
}

fn handle_change_log_event(
    log_messages: Option<Vector<ForwardsUOffset<&str>>>,
) -> Result<Vec<String>, ()> {
    lazy_static! {
        static ref CLRE: Regex = Regex::new(
            r"Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)"
        )
        .unwrap();
    }
    let mut events: Vec<String> = vec![];

    match log_messages {
        Some(lines) => {
            for line in lines {
                let captures = CLRE.captures(line);
                let b64raw = captures.and_then(|c| c.get(1)).map(|c| c.as_str());
                b64raw.map(|raw| events.push((raw).parse().unwrap()));
            }
            if events.is_empty() {
                println!("No events captured!");
                Err(())
            } else {
                Ok(events)
            }
        }
        None => {
            println!("Some plerkle error outside of event parsing/ no log messages");
            Err(())
        }
    }
}

pub fn order_instructions(
    transaction_info: &TransactionInfo,
) -> Vec<(
    solana_sdk::pubkey::Pubkey,
    solana_sdk::instruction::CompiledInstruction,
)> {
    let mut ordered_ixs: Vec<(
        solana_sdk::pubkey::Pubkey,
        solana_sdk::instruction::CompiledInstruction,
    )> = vec![];
    // Get inner instructions.
    let inner_ixs = transaction_info.inner_instructions();

    // Get outer instructions.
    let outer_instructions = match transaction_info.outer_instructions() {
        None => {
            println!("outer instructions deserialization error");
            return ordered_ixs;
        }
        Some(instructions) => instructions,
    };

    // Get account keys.
    let keys = match transaction_info.account_keys() {
        None => {
            println!("account_keys deserialization error");
            return ordered_ixs;
        }
        Some(keys) => keys,
    };

    if let Some(inner_ix_list) = inner_ixs {
        //let inner_ix_list = inner_ixs.as_ref().unwrap().as_slice();
        for inner in inner_ix_list {
            let outer = outer_instructions.get(inner.index() as usize);
            let program_id = keys.get(outer.program_id_index() as usize);
            let program_id = solana_sdk::pubkey::Pubkey::new(program_id.key().unwrap());

            let outer = solana_sdk::instruction::CompiledInstruction::new_from_raw_parts(
                outer.program_id_index(),
                outer.data().unwrap().to_vec(),
                outer.accounts().unwrap().to_vec(),
            );

            ordered_ixs.push((program_id, outer));

            for inner_ix_instance in inner.instructions().unwrap() {
                let inner_program_id = keys.get(inner_ix_instance.program_id_index() as usize);
                let inner_program_id =
                    solana_sdk::pubkey::Pubkey::new(inner_program_id.key().unwrap());

                let inner_ix_instance =
                    solana_sdk::instruction::CompiledInstruction::new_from_raw_parts(
                        inner_ix_instance.program_id_index(),
                        inner_ix_instance.data().unwrap().to_vec(),
                        inner_ix_instance.accounts().unwrap().to_vec(),
                    );
                ordered_ixs.push((inner_program_id, inner_ix_instance));
            }
        }
    } else {
        for instruction in outer_instructions {
            let program_id = keys.get(instruction.program_id_index() as usize);
            let program_id = solana_sdk::pubkey::Pubkey::new(program_id.key().unwrap());
            let instruction = solana_sdk::instruction::CompiledInstruction::new_from_raw_parts(
                instruction.program_id_index(),
                instruction.data().unwrap().to_vec(),
                instruction.accounts().unwrap().to_vec(),
            );
            ordered_ixs.push((program_id, instruction));
        }
    }
    ordered_ixs
}

async fn change_log_event_to_database(
    change_log_event: ChangeLogEvent,
    pid: i64,
    pool: &Pool<Postgres>,
) {
    println!("\tCL tree {:?}", change_log_event.id);
    let txnb = pool.begin().await;
    match txnb {
        Ok(txn) => {
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

fn un_jank_message(hex_str: &String) -> String {
    String::from_utf8(hex::decode(hex_str).unwrap()).unwrap()
}

async fn app_event_to_database(
    app_event: &AppEvent,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), ()> {
    println!("Op: {:?}", app_event.op);
    println!("leaf: {:?}", &app_event.leaf);
    println!("owner: {:?}", &app_event.owner);
    println!("tree_id: {:?}", &app_event.tree_id);
    println!("new_owner: {:?}", &app_event.new_owner);

    if app_event.op == "add" || app_event.op == "tran" || app_event.op == "create" {
        let row = sqlx::query_as::<_, AppSpecificRev>(GET_APPSQL)
            .bind(&un_jank_message(&app_event.message))
            .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
            .fetch_one(pool)
            .await;
        if row.is_ok() {
            let res = row.unwrap();
            if pid < res.revision as i64 {
                return Err(());
            }
        }
    }

    if app_event.op == "add" {
        sqlx::query(SET_APPSQL)
            .bind(&un_jank_message(&app_event.message))
            .bind(&bs58::decode(&app_event.leaf).into_vec().unwrap())
            .bind(&bs58::decode(&app_event.owner).into_vec().unwrap())
            .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
            .bind(&pid)
            .execute(pool)
            .await
            .unwrap();
    } else if app_event.op == "tran" {
        match &app_event.new_owner {
            Some(x) => {
                sqlx::query(SET_APPSQL)
                    .bind(&un_jank_message(&app_event.message))
                    .bind(&bs58::decode(&app_event.leaf).into_vec().unwrap())
                    .bind(&bs58::decode(&x).into_vec().unwrap())
                    .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
                    .bind(&pid)
                    .execute(pool)
                    .await
                    .unwrap();
            }
            None => {
                println!("Received Transfer op with no new_owner");
                return Err(());
            }
        };
    } else if app_event.op == "rm" {
        sqlx::query(DEL_APPSQL)
            .bind(&bs58::decode(&app_event.leaf).into_vec().unwrap())
            .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
            .execute(pool)
            .await
            .unwrap();
    } else if app_event.op == "create" {
        sqlx::query(SET_OWNERSHIP_APPSQL)
            .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
            .bind(&bs58::decode(&app_event.authority).into_vec().unwrap())
            .bind(&pid)
            .execute(pool)
            .await
            .unwrap();
    }

    Ok(())
}

fn pubkey_from_fb_table(
    keys: &Vector<ForwardsUOffset<transaction_info::Pubkey>>,
    index: usize,
) -> String {
    let pubkey = keys.get(index);
    String::from_utf8(pubkey.key().unwrap().to_vec()).unwrap()
}
