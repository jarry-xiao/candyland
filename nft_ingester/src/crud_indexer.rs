use {
    crate::utils::{batch_init_service, pubkey_from_fb_table, un_jank_message, AppSpecificRev},
    anchor_client::anchor_lang::AnchorDeserialize,
    flatbuffers::{ForwardsUOffset, Vector},
    plerkle_serialization::transaction_info_generated::transaction_info::{self},
    serde::Deserialize,
    solana_sdk::keccak,
    sqlx::{self, Pool, Postgres},
    std::fs::File,
};

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

#[derive(Debug, Deserialize)]
pub struct AppSpecificRecord {
    msg: String,
    owner: String,
    leaf: String,
    revision: u32,
}

const SET_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (msg)
                            DO UPDATE SET leaf = excluded.leaf, owner = excluded.owner, tree_id = excluded.tree_id, revision = excluded.revision"#;
pub const SET_OWNERSHIP_APPSQL: &str = r#"INSERT INTO app_specific_ownership (tree_id, authority) VALUES ($1,$2) ON CONFLICT (tree_id)
                            DO UPDATE SET authority = excluded.authority"#;
const GET_APPSQL: &str = "SELECT revision FROM app_specific WHERE msg = $1 AND tree_id = $2";
const DEL_APPSQL: &str = "DELETE FROM app_specific WHERE leaf = $1 AND tree_id = $2";
const BATCH_INSERT_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) 
    SELECT msg, leaf, owner, tree_id, revision
    FROM UNNEST($1,$2,$3,$4,$5) as a(msg, leaf, owner, tree_id, revision)
    RETURNING msg, leaf, owner, tree_id, revision
    "#;

pub async fn handle_gummyroll_crud_instruction(
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

pub async fn batch_insert_app_specific_records(
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
                    println!("Saved CRUD message batch");
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
