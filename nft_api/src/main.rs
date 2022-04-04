use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join, join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, RequestInfo, Router, RouterService};
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};
use std::sync::Mutex;
use std::{convert::Infallible, net::SocketAddr};
use std::borrow::Borrow;
use std::ops::Deref;
use std::process::id;
use std::time::Duration;
use anchor_client::solana_sdk::keccak;
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use gummyroll::ChangeLogEvent;
use crate::events::handle_event;
use tokio_postgres::{NoTls, Error};
use crate::error::ApiError;
use sqlx;
use sqlx::postgres::PgPoolOptions;

mod api;
mod models;
mod events;
mod error;

async fn logger(req: Request<Body>) -> Result<Request<Body>, routerify_json_response::Error> {
    println!(
        "{} {} {}",
        req.remote_addr(),
        req.method(),
        req.uri().path()
    );
    Ok(req)
}

fn router(db: DatabaseConnection) -> Router<Body, routerify_json_response::Error> {
    Router::builder()
        .middleware(Middleware::pre(logger))
        // .get("/assets/:account", api::handle_get_assets)
        //
        // .get("/tree/:tree_id", api::handle_get_tree)
        // .get("/proof/:tree_id/:index", api::handle_get_proof)
        .build()
        .unwrap()
}

#[derive(Default)]
struct AppSpecific {
    op: String,
    message: String,
    leaf: String,
    owner: String,
    tree_id: String,
}

#[tokio::main]
async fn main() {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect("postgres://solana:solana@localhost/solana").await.unwrap();
    let orm_conn = SqlxPostgresConnector::from_sqlx_postgres_pool(pool);

    let set_clsql = "INSERT INTO cl_meta (tree, leaf_idx, seq) VALUES ($1,$2,$3)";
    let set_appsql = "INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON conflict msg DO UPDATE";
    let get_appsql ="SELECT rev FROM app_specific WHERE msg = $1 AND tree_id = $2";
    let del_appsql = "DELETE FROM app_specific WHERE msg = $1 AND tree_id = $2";
    let set_clsql_item = "INSERT INTO cl_items (tree, seq, level, hash, node_index) VALUES ($1,$2,$3,$4)";
    let client = redis::Client::open("redis://redis/").unwrap();
    let router = router(orm_conn);
    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();
    // The address on which the server will be listening.
    let addr = SocketAddr::from(([0, 0, 0, 0], 9090));
    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {}", addr);
    let structured_program_event_service = tokio::spawn(async move {
        let conn_res = client.get_connection();
        let mut conn = conn_res.unwrap();
        let opts = StreamReadOptions::default().block(0).count(1000);
        let mut last_id: String = "$".to_string();
        loop {
            let srr: StreamReadReply = conn.xread_options(&["GMC_OP"], &[&last_id], &opts).unwrap();
            for StreamKey { key, ids } in srr.keys {
                for StreamId { id, map } in ids {
                    let mut app_event = AppSpecific::default();
                    for (k, v) in map {
                        let Value::Data(bytes) = v;
                        let raw_str = String::from_utf8(bytes);
                        if raw_str.is_ok() {
                            if k == "o" {
                                app_event.op = raw_str.unwrap();
                            }
                            if k == "tree_id" {
                                app_event.tree_id = raw_str.unwrap();
                            }
                            if k == "msg" {
                                app_event.message = raw_str.unwrap();
                            }
                            if k == "leaf" {
                                app_event.leaf = raw_str.unwrap();
                            }
                            if k == "owner" {
                                app_event.owner = raw_str.unwrap();
                            }
                        }
                    }
                    println!("\tID {}", id);
                    let pid = id.replace("-", "").parse::<i64>().unwrap();
                    let new_owner = map.get("new_owner").and_then(|x| {
                        let Value::Data(bytes) = x.to_owned();
                        String::from_utf8(bytes).ok()
                    });
                    if app_event.op == "add" || app_event.op == "tran" {
                        let rev: i64 = sqlx::query_as(get_appsql).bind(&app_event.message).bind(&app_event.tree_id).fetch_one(&pool).await.unwrap();
                        if pid < rev as i64 {
                            continue;
                        }
                    }
                    if app_event.op == "add" {
                        sqlx::query(set_appsql)
                                            .bind(&app_event.message)
                                            .bind(&app_event.leaf)
                                            .bind(&app_event.owner)
                                            .bind(&app_event.tree_id)
                                            .bind(&pid)
                            .execute(&pool).await.unwrap();

                    }
                    if app_event.op == "tran" {
                        new_owner.map(|x| async {
                            sqlx::query(set_appsql)
                                .bind(&app_event.message)
                                .bind(&app_event.leaf)
                                .bind(&x)
                                .bind(&app_event.tree_id)
                                .bind(&pid)
                                .execute(&pool).await.unwrap();
                        });
                    }
                    if app_event.op == "rm" {
                        sqlx::query(set_appsql)
                            .bind(&app_event.message)
                            .bind(&app_event.tree_id)
                            .execute(&pool).await.unwrap();
                    }
                }
            }
        }
    });
    let cl_service = tokio::spawn(async move {
        let conn_res = client.get_connection();
        let mut conn = conn_res.unwrap();
        let opts = StreamReadOptions::default().block(0).count(1000);
        let mut last_id: String = "$".to_string();
        loop {
            let srr: StreamReadReply = conn.xread_options(&["GM_CL"], &[&last_id], &opts).unwrap();
            for StreamKey { key, ids } in srr.keys {
                println!("Stream {}", key);
                for StreamId { id, map } in ids {
                    println!("\tID {}", id);
                    let pid = id.replace("-", "").parse::<i64>().unwrap();
                    let data = map.get("data");
                    if data.is_none() {
                        continue;
                    }
                    let Value::Data(bytes) = data.unwrap().to_owned();
                    let raw_str = String::from_utf8(bytes);
                    if !raw_str.is_ok() {
                        continue;
                    }
                    let Ok(change_log) = raw_str.map_err(|serr| {
                        ApiError::ChangeLogEventMalformed
                    })
                        .and_then(|o| {
                            let d: Result<ChangeLogEvent, ApiError> = handle_event(o);
                            d
                        });
                    let mut txnb = pool.begin().await;
                    match txnb {
                        Ok(txn) => {
                            let mut i: i64 = 0;
                            for (node, node_index) in change_log.path.into_iter() {
                                sqlx::query(set_clsql_item)
                                    .bind(&change_log.id.as_ref())
                                    .bind(&pid)
                                    .bind(&i)
                                    .bind(&node.inner.as_ref())
                                    .bind(&(node_index as i64))
                                    .execute(&pool).await.unwrap();
                                i += 1;
                            }
                            match txn.commit().await {
                                Ok(r) => {
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
                    last_id = id.to_owned();
                }
            }
        }
    });
    match join3(server, cl_service, structured_program_event_service).await {
        (Err(err), _, _) => {
            eprintln!("Server error: {}", err);
        }
        (_, Err(err), _) => {
            eprintln!("Change Log Service error: {}", err);
        }
        (_, _, Err(err)) => {
            eprintln!("Structure App Service error: {}", err);
        }
        _ => {
            eprintln!("Closing");
        }
    }
}
