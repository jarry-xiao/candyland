extern crate core;

use hyper::{Body, Client, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService};


use std::{net::SocketAddr, thread};
use anchor_client::anchor_lang::prelude::Pubkey;
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};


use gummyroll::{ChangeLogEvent, PathNode};


use sqlx;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use tokio::task;
use nft_api_lib::error::*;
use nft_api_lib::events::handle_event;

#[derive(Default)]
struct AppEvent {
    op: String,
    message: String,
    leaf: String,
    owner: String,
    tree_id: String,
    authority: String,
}

const SET_APPSQL: &str = r#"INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (msg)
                            DO UPDATE SET leaf = excluded.leaf, owner = excluded.owner, tree_id = excluded.tree_id, revision = excluded.revision"#;
const SET_OWNERSHIP_APPSQL: &str = r#"INSERT INTO app_specific_ownership (tree_id, authority) VALUES ($1,$2) ON CONFLICT (tree_id)
                            DO UPDATE SET authority = excluded.authority"#;
const GET_APPSQL: &str = "SELECT revision FROM app_specific WHERE msg = $1 AND tree_id = $2";
const DEL_APPSQL: &str = "DELETE FROM app_specific WHERE msg = $1 AND tree_id = $2";
const SET_CLSQL_ITEM: &str = "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5)";

#[derive(sqlx::FromRow, Clone, Debug)]
struct AppSpecificRev {
    revision: i64,
}

pub async fn cl_service(ids: &Vec<StreamId>, pool: &Pool<Postgres>) -> String {
    let mut last_id = "".to_string();
    for StreamId { id, map } in ids {
        println!("\tCL STREAM ID {}", id);
        let pid = id.replace("-", "").parse::<i64>().unwrap();

        let data = map.get("data");

        if data.is_none() {
            println!("\tNo Data");
            continue;
        }

        if let Value::Data(bytes) = data.unwrap().to_owned() {
            let raw_str = String::from_utf8(bytes);
            if !raw_str.is_ok() {
                continue;
            }
            let change_log_res = raw_str.map_err(|_serr| {
                ApiError::ChangeLogEventMalformed
            })
                .and_then(|o| {
                    let d: Result<ChangeLogEvent, ApiError> = handle_event(o);
                    d
                });
            if change_log_res.is_err() {
                println!("\tBad Data");
                continue;
            }
            let change_log = change_log_res.unwrap();
            println!("\tCL tree {:?}", change_log.id);
            let txnb = pool.begin().await;
            match txnb {
                Ok(txn) => {
                    let mut i: i64 = 0;
                    for p in change_log.path.into_iter() {
                        let tree_id = change_log.id.as_ref();
                        let f = sqlx::query(SET_CLSQL_ITEM)
                            .bind(&tree_id)
                            .bind(&pid + i)
                            .bind(&i)
                            .bind(&p.node.inner.as_ref())
                            .bind(&(p.index as i64))
                            .execute(pool).await;
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
        last_id = id.clone();
    }
    last_id
}

pub async fn structured_program_event_service(ids: &Vec<StreamId>, pool: &Pool<Postgres>) -> String {
    let mut last_id = "".to_string();
    for StreamId { id, map } in ids {
        println!("\thello jarry....");
        let mut app_event = AppEvent::default();
        for (k, v) in map.to_owned() {
            if let Value::Data(bytes) = v {
                let raw_str = String::from_utf8(bytes);
                if raw_str.is_ok() {
                    if k == "op" {
                        app_event.op = raw_str.unwrap();
                    } else if k == "tree_id" {
                        app_event.tree_id = raw_str.unwrap();
                    } else if k == "msg" {
                        app_event.message = raw_str.unwrap();
                    } else if k == "leaf" {
                        app_event.leaf = raw_str.unwrap();
                    } else if k == "owner" {
                        app_event.owner = raw_str.unwrap();
                    } else if k == "authority" {
                        app_event.authority = raw_str.unwrap();
                    }
                }
            }
        }

        let pid = id.replace("-", "").parse::<i64>().unwrap();
        let new_owner = map.get("new_owner").and_then(|x| {
            if let Value::Data(bytes) = x.to_owned() {
                String::from_utf8(bytes).ok()
            } else {
                None
            }
        });
        if app_event.op == "add" || app_event.op == "tran" || app_event.op == "create" {
            let row = sqlx::query_as::<_, AppSpecificRev>(GET_APPSQL)
                .bind(&un_jank_message(&app_event.message))
                .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
                .fetch_one(pool)
                .await;
            if row.is_ok() {
                let res = row.unwrap();
                if pid < res.revision as i64 {
                    continue;
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
                .execute(pool).await.unwrap();
        } else if app_event.op == "tran" {
            new_owner.map(|x| async move {
                sqlx::query(SET_APPSQL)
                    .bind(&un_jank_message(&app_event.message))
                    .bind(&bs58::decode(&app_event.leaf).into_vec().unwrap())
                    .bind(&bs58::decode(&x).into_vec().unwrap())
                    .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
                    .bind(&pid)
                    .execute(pool).await.unwrap();
            });
        } else if app_event.op == "rm" {
            sqlx::query(DEL_APPSQL)
                .bind(&un_jank_message(&app_event.message))
                .bind(&bs58::decode(app_event.tree_id).into_vec().unwrap())
                .execute(pool).await.unwrap();
        } else if app_event.op == "create" {
            sqlx::query(SET_OWNERSHIP_APPSQL)
                .bind(&bs58::decode(&app_event.tree_id).into_vec().unwrap())
                .bind(&bs58::decode(&app_event.authority).into_vec().unwrap())
                .bind(&pid)
                .execute(pool).await.unwrap();
        }
        last_id = id.clone();
    }
    last_id
}

fn un_jank_message(hex_str: &String) -> String {
    String::from_utf8(hex::decode(hex_str).unwrap()).unwrap()
}

#[tokio::main]
async fn main() {
    let client = redis::Client::open("redis://redis/").unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://solana:solana@db/solana").await.unwrap();
    let mut cl_last_id: String = ">".to_string();
    let mut gm_last_id: String = ">".to_string();
    let conn_res = client.get_connection();
    let mut conn = conn_res.unwrap();
    let streams = vec!["GM_CL", "GMC_OP"];
    let group_name = "ingester";
    for key in &streams {
        let created: Result<(), _> = conn.xgroup_create_mkstream(*key, group_name, "$");
        if let Err(e) = created {
            println!("Group already exists: {:?}", e)
        }
    }
    loop {
        let opts = StreamReadOptions::default().block(1000).count(100000).group(group_name, "lelelelle");
        let srr: StreamReadReply = conn.xread_options(streams.as_slice(), &[&cl_last_id, &gm_last_id], &opts).unwrap();
        for StreamKey { key, ids } in srr.keys {
            println!("{}", key);
            if key == "GM_CL" {
                cl_service(&ids, &pool).await;
            } else if key == "GMC_OP" {
                structured_program_event_service(&ids, &pool).await;
            }
        }
    }
}
