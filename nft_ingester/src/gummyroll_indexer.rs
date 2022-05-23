use {
    crate::filter_events_from_logs,
    gummyroll::state::change_log::ChangeLogEvent,
    nft_ingester::events::handle_event,
    serde::Deserialize,
    sqlx::{self, Pool, Postgres},
    std::fs::File,
};

const SET_CLSQL_ITEM: &str =
    "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5)";
const BATCH_INSERT_CLSQL: &str = r#"INSERT INTO cl_items (tree, seq, level, hash, node_idx) 
    SELECT tree, seq, level, hash, node_idx
    FROM UNNEST($1,$2,$3,$4,$5) as a(tree, seq, level, hash, node_idx) 
    RETURNING tree, seq, level, hash, node_idx
    "#;

#[derive(Debug, Deserialize)]
pub struct CLRecord {
    node_idx: u32,
    level: u32,
    seq: u32,
    hash: String,
}

pub async fn handle_gummyroll_instruction(
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

pub async fn batch_insert_cl_records(
    pool: &Pool<Postgres>,
    records: &Vec<CLRecord>,
    tree_id: &str,
) {
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
