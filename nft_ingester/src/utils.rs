use {
    crate::{
        crud_indexer::{insert_csv_metadata, SET_OWNERSHIP_APPSQL},
        gummyroll_indexer::insert_csv_cl,
    },
    flatbuffers::{ForwardsUOffset, Vector},
    lazy_static::lazy_static,
    nft_ingester::error::IngesterError,
    plerkle_serialization::transaction_info_generated::transaction_info::{self, TransactionInfo},
    regex::Regex,
    solana_sdk::pubkey::Pubkey,
    sqlx::{self, Pool, Postgres},
    std::fs::File,
    std::io::Write,
};

#[derive(sqlx::FromRow, Clone, Debug)]
pub struct AppSpecificRev {
    pub revision: i64,
}

pub async fn write_assets_to_file(uri: &str, tree_id: &str, key: &str) -> Result<String, IngesterError> {
    println!("Requesting to see arweave link for {}", key);
    let fname = format!("{}-{}.csv", tree_id, key);
    let body = reqwest::get(uri).await?.text().await?;
    let mut file = File::create(&fname)?;
    println!("{:?}", body.len());
    file.write_all(body.as_bytes())?;
    println!("Wrote response to {}", &fname);
    Ok(fname.to_string())
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

pub fn un_jank_message(hex_str: &String) -> String {
    String::from_utf8(hex::decode(hex_str).unwrap()).unwrap()
}

pub fn pubkey_from_fb_table(
    keys: &Vector<ForwardsUOffset<transaction_info::Pubkey>>,
    index: usize,
) -> String {
    let pubkey = keys.get(index);
    Pubkey::new(pubkey.key().unwrap()).to_string()
}

pub fn filter_events_from_logs(log_messages: &Vec<String>) -> Result<Vec<String>, ()> {
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
