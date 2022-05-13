use {
    anchor_client::anchor_lang::AnchorDeserialize,
    bubblegum, csv,
    flatbuffers::{ForwardsUOffset, Vector},
    gummyroll::state::change_log::ChangeLogEvent,
    lazy_static::lazy_static,
    messenger::{Messenger, ACCOUNT_STREAM, BLOCK_STREAM, SLOT_STREAM, TRANSACTION_STREAM},
    nft_api_lib::error::ApiError,
    nft_api_lib::events::handle_event,
    plerkle::redis_messenger::RedisMessenger,
    plerkle_serialization::transaction_info_generated::transaction_info::{
        self, root_as_transaction_info, TransactionInfo,
    },
    regex::Regex,
    reqwest,
    serde::Deserialize,
    solana_sdk::keccak,
    solana_sdk::pubkey::Pubkey,
    sqlx::{self, postgres::PgPoolOptions, Pool, Postgres},
    std::fs::File,
    std::io::Write,
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

#[derive(Default)]
struct AppEvent {
    op: String,
    message: String,
    leaf: String,
    owner: String,
    new_owner: Option<String>,
    tree_id: String,
    authority: String,
    metadata_db_uri: String,
    changelog_db_uri: String,
}

const SET_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (msg)
                            DO UPDATE SET leaf = excluded.leaf, owner = excluded.owner, tree_id = excluded.tree_id, revision = excluded.revision"#;
const SET_OWNERSHIP_APPSQL: &str = r#"INSERT INTO app_specific_ownership (tree_id, authority) VALUES ($1,$2) ON CONFLICT (tree_id)
                            DO UPDATE SET authority = excluded.authority"#;

const SET_NFT_APPSQL: &str = r#"
    INSERT INTO nft_metadata (
        leaf,
        tree_id,
        revision,   
        owner,      
        delegate,   
        nonce,      
        name,       
        symbol,     
        uri
    )
    VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) 
    "#;

const GET_APPSQL: &str = "SELECT revision FROM app_specific WHERE msg = $1 AND tree_id = $2";
const DEL_APPSQL: &str = "DELETE FROM app_specific WHERE leaf = $1 AND tree_id = $2";
const SET_CLSQL_ITEM: &str =
    "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5)";
const BATCH_INSERT_CLSQL: &str = r#"INSERT INTO cl_items (tree, seq, level, hash, node_idx) 
    SELECT tree, seq, level, hash, node_idx
    FROM UNNEST($1,$2,$3,$4,$5) as a(tree, seq, level, hash, node_idx) 
    RETURNING tree, seq, level, hash, node_idx
    "#;
const BATCH_INSERT_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) 
    SELECT msg, leaf, owner, tree_id, revision
    FROM UNNEST($1,$2,$3,$4,$5) as a(msg, leaf, owner, tree_id, revision)
    RETURNING msg, leaf, owner, tree_id, revision
    "#;

#[derive(sqlx::FromRow, Clone, Debug)]
struct AppSpecificRev {
    revision: i64,
}

pub async fn write_assets_to_file(uri: &str, tree_id: &str, key: &str) -> Result<String, ApiError> {
    println!("Requesting to see arweave link for {}", key);
    let fname = format!("{}-{}.csv", tree_id, key);
    let body = reqwest::get(uri).await?.text().await?;
    let mut file = File::create(&fname)?;
    println!("{:?}", body.len());
    file.write_all(body.as_bytes())?;
    println!("Wrote response to {}", &fname);
    Ok(fname.to_string())
}

#[derive(Debug, Deserialize)]
struct CLRecord {
    node_idx: u32,
    level: u32,
    seq: u32,
    hash: String,
}

#[derive(Debug, Deserialize)]
struct AppSpecificRecord {
    msg: String,
    owner: String,
    leaf: String,
    revision: u32,
}

async fn batch_insert_app_specific_records(
    pool: &Pool<Postgres>,
    records: &Vec<AppSpecificRecord>,
    tree_id: &str,
) {
    let mut tree_ids: Vec<Vec<u8>> = vec![];
    let mut owners: Vec<Vec<u8>> = vec![];
    let mut revisions: Vec<u32> = vec![];
    let mut msgs: Vec<String> = vec![];
    let mut leaves: Vec<Vec<u8>> = vec![];

    for record in records.iter() {
        tree_ids.push(bs58::decode(&tree_id).into_vec().unwrap());
        owners.push(bs58::decode(&record.owner).into_vec().unwrap());
        revisions.push(record.revision);
        msgs.push(record.msg.clone());
        leaves.push(bs58::decode(&record.leaf).into_vec().unwrap());
    }

    let txnb = pool.begin().await;
    match txnb {
        Ok(txn) => {
            let f = sqlx::query(BATCH_INSERT_APPSQL)
                .bind(&msgs)
                .bind(&leaves)
                .bind(&owners)
                .bind(&tree_ids)
                .bind(&revisions)
                .execute(pool)
                .await;

            if f.is_err() {
                println!("Error: {:?}", f.err().unwrap());
            }

            match txn.commit().await {
                Ok(_r) => {
                    println!("Saved CL");
                }
                Err(e) => {
                    println!("{}", e.to_string())
                }
            }
        }
        Err(e) => {
            println!("{}", e.to_string())
        }
    }
}

async fn batch_insert_cl_records(pool: &Pool<Postgres>, records: &Vec<CLRecord>, tree_id: &str) {
    let mut tree_ids: Vec<Vec<u8>> = vec![];
    let mut node_idxs: Vec<u32> = vec![];
    let mut seq_nums: Vec<u32> = vec![];
    let mut levels: Vec<u32> = vec![];
    let mut hashes: Vec<Vec<u8>> = vec![];

    for record in records.iter() {
        tree_ids.push(bs58::decode(&tree_id).into_vec().unwrap());
        node_idxs.push(record.node_idx);
        levels.push(record.level);
        seq_nums.push(record.seq);
        hashes.push(bs58::decode(&record.hash).into_vec().unwrap());
    }

    let txnb = pool.begin().await;
    match txnb {
        Ok(txn) => {
            let f = sqlx::query(BATCH_INSERT_CLSQL)
                .bind(&tree_ids)
                .bind(&seq_nums)
                .bind(&levels)
                .bind(&hashes)
                .bind(&node_idxs)
                .execute(pool)
                .await;

            if f.is_err() {
                println!("Error: {:?}", f.err().unwrap());
            }

            match txn.commit().await {
                Ok(_r) => {
                    println!("Saved CL");
                }
                Err(e) => {
                    println!("{}", e.to_string())
                }
            }
        }
        Err(e) => {
            println!("{}", e.to_string())
        }
    }
}

pub async fn insert_csv_cl(pool: &Pool<Postgres>, fname: &str, batch_size: usize, tree_id: &str) {
    let tmp_file = File::open(fname).unwrap();
    let mut reader = csv::Reader::from_reader(tmp_file);

    let mut batch = vec![];
    let mut num_batches = 0;
    for result in reader.deserialize() {
        let record = result.unwrap();
        batch.push(record);

        if batch.len() == batch_size {
            println!("Executing batch write: {}", num_batches);
            batch_insert_cl_records(pool, &batch, tree_id).await;
            batch = vec![];
            num_batches += 1;
        }
    }
    if batch.len() > 0 {
        batch_insert_cl_records(pool, &batch, tree_id).await;
        num_batches += 1;
    }
    println!("Uploaded to db in {} batches", num_batches);
}

pub async fn insert_csv_metadata(
    pool: &Pool<Postgres>,
    fname: &str,
    batch_size: usize,
    tree_id: &str,
) {
    let tmp_file = File::open(fname).unwrap();
    let mut reader = csv::Reader::from_reader(tmp_file);

    let mut batch = vec![];
    let mut num_batches = 0;
    for result in reader.deserialize() {
        let record = result.unwrap();
        println!("Record: {:?}", record);
        batch.push(record);

        if batch.len() == batch_size {
            println!("Executing batch write: {}", num_batches);
            batch_insert_app_specific_records(pool, &batch, tree_id).await;
            batch = vec![];
            num_batches += 1;
        }
    }
    if batch.len() > 0 {
        batch_insert_app_specific_records(pool, &batch, tree_id).await;
        num_batches += 1;
    }
    println!("Uploaded to db in {} batches", num_batches);
}

pub async fn batch_init_service(
    pool: &Pool<Postgres>,
    authority: &str,
    tree_id: &str,
    changelog_db_uri: &str,
    metadata_db_uri: &str,
    pid: i64,
) {
    let changelog_fname: String = write_assets_to_file(&changelog_db_uri, &tree_id, "changelog")
        .await
        .unwrap();
    let metadata_fname: String = write_assets_to_file(&metadata_db_uri, &tree_id, "metadata")
        .await
        .unwrap();

    insert_csv_cl(pool, &changelog_fname, 100, &tree_id).await;
    println!("Wrote changelog file to db");
    insert_csv_metadata(pool, &metadata_fname, 100, &tree_id).await;
    println!("Wrote metadata file to db");

    println!(
        "Issuing authority update for tree: {} auth: {}",
        &tree_id, &authority
    );
    let tree_bytes: Vec<u8> = bs58::decode(&tree_id).into_vec().unwrap();
    let auth_bytes: Vec<u8> = bs58::decode(&authority).into_vec().unwrap();

    sqlx::query(SET_OWNERSHIP_APPSQL)
        .bind(&tree_bytes)
        .bind(&auth_bytes)
        .bind(&pid)
        .execute(pool)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() {
    // Setup Redis Messenger.
    let mut messenger = RedisMessenger::new().unwrap();
    messenger.add_stream(ACCOUNT_STREAM);
    messenger.add_stream(SLOT_STREAM);
    messenger.add_stream(TRANSACTION_STREAM);
    messenger.add_stream(BLOCK_STREAM);

    // Setup Postgres.
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://solana:solana@db/solana")
        .await
        .unwrap();

    // Service streams.
    loop {
        if let Ok(_) = messenger.recv() {
            if let Ok(data) = messenger.get(TRANSACTION_STREAM) {
                handle_transaction(data, &pool).await;
            }

            // Ignore these for now.
            let _ = messenger.get(ACCOUNT_STREAM);
            let _ = messenger.get(SLOT_STREAM);
            let _ = messenger.get(BLOCK_STREAM);
        }
    }
}

pub async fn handle_transaction(data: Vec<(i64, &[u8])>, pool: &Pool<Postgres>) {
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

        // Handle change log events
        if keys
            .iter()
            .any(|pubkey| pubkey.key().unwrap() == program_ids::gummyroll().to_bytes())
        {
            println!("Found GM CL event");
            // Get vector of change log events.
        }

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
                (program, instruction) if program == program_ids::gummyroll() => {
                    handle_gummyroll_instruction(&parsed_log.1, pid, pool).await;
                }
                (program, instruction) if program == program_ids::gummyroll_crud() => {
                    handle_gummyroll_crud_instruction(&instruction, &keys, pid, pool)
                        .await
                        .unwrap();
                }
                (program, instruction) if program == program_ids::bubblegum() => {
                    handle_bubblegum_instruction(&instruction, &keys, pid, pool)
                        .await
                        .unwrap();
                }
                _ => {}
            }
        }
    }
}

async fn handle_gummyroll_instruction(
    logs: &Vec<String>,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), ()> {
    let change_log_event_vec = if let Ok(change_log_event_vec) = filter_events_from_logs(logs) {
        change_log_event_vec
    } else {
        println!("Could find emitted program data");
        return Err(());
    };

    // Parse each change log event found in logs
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
    Ok(())
}

async fn handle_bubblegum_instruction(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    keys: &Vector<'_, ForwardsUOffset<transaction_info::Pubkey<'_>>>,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), ()> {
    match bubblegum::get_instruction_type(&instruction.data) {
        bubblegum::InstructionName::Transfer => {
            // Insert metadata into database here
            println!("Ah yes! a transfer");
        }
        bubblegum::InstructionName::Mint => {
            // Insert metadata into database here
            println!("Ah yes! a mint");

            /*
            let mint_auth = pubkey_from_fb_table(keys, instruction.accounts[0] as usize);
            let auth = pubkey_from_fb_table(keys, instruction.accounts[1] as usize);
            */
            let owner = pubkey_from_fb_table(keys, instruction.accounts[4] as usize);
            let delegate = pubkey_from_fb_table(keys, instruction.accounts[4] as usize);
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[6] as usize);
            // Get authority.
            let data = instruction.data[8..].to_owned();
            let data_buf = &mut data.as_slice();
            let ix: bubblegum::instruction::Mint =
                bubblegum::instruction::Mint::deserialize(data_buf).unwrap();
            let metadata = ix.message;
            let leaf = bubblegum::hash_metadata(&metadata).unwrap();
            println!(
                "Metadata info: {} {} {} {}",
                &metadata.name,
                metadata.seller_fee_basis_points,
                metadata.primary_sale_happened,
                metadata.is_mutable,
            );
            sqlx::query(SET_NFT_APPSQL)
                .bind(&leaf.to_vec())
                .bind(&bs58::decode(&tree_id).into_vec().unwrap())
                .bind(&pid)
                .bind(&bs58::decode(&owner).into_vec().unwrap())
                .bind(&bs58::decode(&delegate).into_vec().unwrap())
                // nonce
                .bind(0 as i64)
                // name
                .bind(&metadata.name)
                // symbol
                .bind(&metadata.symbol)
                // uri
                .bind(&metadata.uri)
                // sellerfeebasispoints
                .bind(metadata.seller_fee_basis_points as u32)
                // primarysalehappened
                .bind(metadata.primary_sale_happened)
                // isMutable
                .bind(metadata.is_mutable)
                .execute(pool)
                .await
                .unwrap();
            println!("Inserted!");
        }
        bubblegum::InstructionName::Decompress => {
            // This is actually just a remove?
            println!("Ah yes! a decompress");
        }
        _ => println!("unknown, or don't care"),
    }
    Ok(())
}

async fn handle_gummyroll_crud_instruction(
    instruction: &solana_sdk::instruction::CompiledInstruction,
    keys: &Vector<'_, ForwardsUOffset<transaction_info::Pubkey<'_>>>,
    pid: i64,
    pool: &Pool<Postgres>,
) -> Result<(), ()> {
    let mut app_event = AppEvent::default();
    // If we populated an app event, write it to the database.
    match gummyroll_crud::get_instruction_type(&instruction.data) {
        gummyroll_crud::InstructionName::CreateTree => {
            // Get tree ID.
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[3] as usize);

            // Get authority.
            let auth = pubkey_from_fb_table(keys, instruction.accounts[0] as usize);

            // Populate app event.
            app_event.op = String::from("create");
            app_event.tree_id = tree_id;
            app_event.authority = auth;
        }
        gummyroll_crud::InstructionName::CreateTreeWithRoot => {
            // Get tree ID.
            println!("Captured tree with root");
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[3] as usize);

            // Get authority.
            let auth = pubkey_from_fb_table(keys, instruction.accounts[0] as usize);

            // Get data.
            let data = instruction.data[8..].to_owned();
            let data_buf = &mut data.as_slice();
            let ix: gummyroll_crud::instruction::CreateTreeWithRoot =
                gummyroll_crud::instruction::CreateTreeWithRoot::deserialize(data_buf).unwrap();

            app_event.op = String::from("create_batch");
            app_event.tree_id = tree_id;
            app_event.authority = auth;
            app_event.metadata_db_uri = String::from_utf8(ix.metadata_db_uri.to_vec()).unwrap();
            app_event.changelog_db_uri = String::from_utf8(ix.changelog_db_uri.to_vec()).unwrap();
        }
        gummyroll_crud::InstructionName::Add => {
            // Get data.
            let data = instruction.data[8..].to_owned();
            let data_buf = &mut data.as_slice();
            let add: gummyroll_crud::instruction::Add =
                gummyroll_crud::instruction::Add::deserialize(data_buf).unwrap();

            // Get tree ID.
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[3] as usize);

            // Get owner from index 0.
            let owner = pubkey_from_fb_table(keys, instruction.accounts[0] as usize);

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
                gummyroll_crud::instruction::Transfer::deserialize(data_buf).unwrap();

            // Get tree ID.
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[3] as usize);

            // Get owner from index 4.
            let owner = pubkey_from_fb_table(keys, instruction.accounts[4] as usize);

            // Get new owner from index 5.
            let new_owner = pubkey_from_fb_table(keys, instruction.accounts[5] as usize);

            // Get message and leaf.
            let hex_message = hex::encode(&add.message);
            let leaf = keccak::hashv(&[&new_owner.as_bytes(), add.message.as_slice()]);

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
            let tree_id = pubkey_from_fb_table(keys, instruction.accounts[3] as usize);

            // Get owner from index 0.
            let owner = pubkey_from_fb_table(keys, instruction.accounts[0] as usize);

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

    if app_event.op.len() > 0 {
        let _result = app_event_to_database(&app_event, pid, pool).await;
    }
    Ok(())
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

fn filter_events_from_logs(log_messages: &Vec<String>) -> Result<Vec<String>, ()> {
    lazy_static! {
        static ref CLRE: Regex = Regex::new(
            r"Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)"
        )
        .unwrap();
    }
    let mut events: Vec<String> = vec![];

    for line in log_messages {
        let captures = CLRE.captures(&line);
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
    let inner_ix_list = transaction_info.inner_instructions();

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

    for (i, instruction) in outer_instructions.iter().enumerate() {
        let program_id = keys.get(instruction.program_id_index() as usize);
        let program_id = solana_sdk::pubkey::Pubkey::new(program_id.key().unwrap());
        let instruction = solana_sdk::instruction::CompiledInstruction::new_from_raw_parts(
            instruction.program_id_index(),
            instruction.data().unwrap().to_vec(),
            instruction.accounts().unwrap().to_vec(),
        );
        ordered_ixs.push((program_id, instruction));

        if let Some(inner_ixs) = get_inner_ixs(inner_ix_list, i) {
            for inner_ix_instance in inner_ixs.instructions().unwrap() {
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
    }

    ordered_ixs
}

fn get_inner_ixs<'a>(
    inner_ixs: Option<Vector<'a, ForwardsUOffset<transaction_info::InnerInstructions<'_>>>>,
    outer_index: usize,
) -> Option<transaction_info::InnerInstructions<'a>> {
    match inner_ixs {
        Some(inner_ix_list) => {
            for inner_ixs in inner_ix_list {
                if inner_ixs.index() == (outer_index as u8) {
                    return Some(inner_ixs);
                }
            }
            None
        }
        None => None,
    }
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
    } else if app_event.op == "create_batch" {
        println!("Captured batch event");
        batch_init_service(
            &pool,
            &app_event.tree_id,
            &app_event.authority,
            &app_event.changelog_db_uri,
            &app_event.metadata_db_uri,
            pid,
        )
        .await;
    }

    Ok(())
}

fn pubkey_from_fb_table(
    keys: &Vector<ForwardsUOffset<transaction_info::Pubkey>>,
    index: usize,
) -> String {
    let pubkey = keys.get(index);
    Pubkey::new(pubkey.key().unwrap()).to_string()
}
