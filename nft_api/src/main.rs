use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService};


use std::{net::SocketAddr};
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};


use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use gummyroll::ChangeLogEvent;


use sqlx;
use sqlx::postgres::PgPoolOptions;

mod models;
mod events;
mod error;

use events::handle_event;
use error::ApiError;
use models::prelude::AppSpecific;
use sea_orm::{entity::*, query::*};
use tokio::{join, task};
use crate::models::{app_specific, cl_items};
use crate::models::prelude::ClItems;

async fn logger(req: Request<Body>) -> Result<Request<Body>, routerify_json_response::Error> {
    println!(
        "{} {} {}",
        req.remote_addr(),
        req.method(),
        req.uri().path()
    );
    Ok(req)
}

async fn handle_get_assets(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &DatabaseConnection = req.data::<DatabaseConnection>().unwrap();
    let owner = req.param("account").unwrap();
    let res = AppSpecific::find()
        .filter(app_specific::Column::Owner.eq(owner.to_owned()))
        .all(db)
        .await;
    if res.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, res.err().unwrap().to_string());
    }
    json_success_resp(&res.unwrap())
}

async fn handle_get_proof(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &DatabaseConnection = req.data::<DatabaseConnection>().unwrap();
    let tree_id = req.param("tree_id").unwrap();
    let index = req.param("index").unwrap();
    let res = ClItems::find()
        .filter(
            Condition::all()
                .add(cl_items::Column::Tree.eq(tree_id.to_owned()))
                .add(cl_items::Column::NodeIdx.gte(index.to_owned()))
            )
        .order_by_asc(cl_items::Column::Level)
        .all(db)
        .await;
    if res.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, res.err().unwrap().to_string());
    }
    let items:Vec<cl_items::Model> = res.unwrap();





    json_success_resp(&res.unwrap())
}

fn router(db: DatabaseConnection) -> Router<Body, routerify_json_response::Error> {
    Router::builder()
        .middleware(Middleware::pre(logger))
        .data(db)
        .get("/assets/:account", handle_get_assets)
        .get("/tree/:tree_id", api::handle_get_tree)
        .get("/proof/:tree_id/:index", api::handle_get_proof)
        .build()
        .unwrap()
}

#[derive(Default)]
struct AppEvent {
    op: String,
    message: String,
    leaf: String,
    owner: String,
    tree_id: String,
}

#[tokio::main]
async fn main() {
    let main_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://solana:solana@db/solana").await.unwrap();
    let orm_conn = SqlxPostgresConnector::from_sqlx_postgres_pool(main_pool);
    let set_appsql = "INSERT INTO app_specific (msg, leaf, owner, tree_id, revision) VALUES ($1,$2,$3,$4,$5) ON conflict msg DO UPDATE";
    let get_appsql = "SELECT revision FROM app_specific WHERE msg = $1 AND tree_id = $2 RETURNING revision";
    let del_appsql = "DELETE FROM app_specific WHERE msg = $1 AND tree_id = $2";
    let set_clsql_item = "INSERT INTO cl_items (tree, seq, level, hash, node_idx) VALUES ($1,$2,$3,$4,$5)";

    let router = router(orm_conn);
    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();
    // The address on which the server will be listening.
    let addr = SocketAddr::from(([0, 0, 0, 0], 9090));
    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {}", addr);
    let structured_program_event_service = async move {
        let client = redis::Client::open("redis://redis/").unwrap();
        let pool = &PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://solana:solana@db/solana").await.unwrap();
        let conn_res = client.get_connection();
        let mut conn = conn_res.unwrap();
        let opts = StreamReadOptions::default().block(0).count(1000);
        let mut last_id: String = "$".to_string();
        loop {
            println!("GM");
            let srr: StreamReadReply = conn.xread_options(&["GMC_OP"], &[&last_id], &opts).unwrap();
            for StreamKey { key: _, ids } in srr.keys {
                for StreamId { id, map } in ids {
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
                    if app_event.op == "add" || app_event.op == "tran" {
                        let row: (i64, ) = sqlx::query_as(get_appsql)
                            .bind(&app_event.message)
                            .bind(&app_event.tree_id)
                            .fetch_one(pool).await.unwrap();
                        if pid < row.0 as i64 {
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
                            .execute(pool).await.unwrap();
                    } else if app_event.op == "tran" {
                        new_owner.map(|x| async move {
                            sqlx::query(set_appsql)
                                .bind(&app_event.message)
                                .bind(&app_event.leaf)
                                .bind(&x)
                                .bind(&app_event.tree_id)
                                .bind(&pid)
                                .execute(pool).await.unwrap();
                        });
                    } else if app_event.op == "rm" {
                        sqlx::query(del_appsql)
                            .bind(&app_event.message)
                            .bind(&app_event.tree_id)
                            .execute(pool).await.unwrap();
                    }
                    last_id = id;
                }
            }
        }
    };
    let cl_service = async move {
        let client = redis::Client::open("redis://redis/").unwrap();
        let pool = &PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://solana:solana@db/solana").await.unwrap();
        let conn_res = client.get_connection();
        let mut conn = conn_res.unwrap();
        let opts = StreamReadOptions::default().block(0).count(1000);
        let mut last_id: String = "$".to_string();
        loop {
            println!("CL {}", last_id);
            let srr: StreamReadReply = conn.xread_options(&["GM_CL"], &[&last_id], &opts).unwrap();
            for StreamKey { key, ids } in srr.keys {
                println!("\tCL STREAM");
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
                        println!("\tSTR {:?}", raw_str);
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
                        println!("\tCL tree {:?} path {:?}", change_log.id, change_log.path);
                        let txnb = pool.begin().await;
                        println!("{:?}", txnb);
                        match txnb {
                            Ok(txn) => {
                                let mut i: i64 = 0;
                                for (node, node_index) in change_log.path.into_iter() {
                                    let f = sqlx::query(set_clsql_item)
                                        .bind(&change_log.id.as_ref())
                                        .bind(&pid+i)
                                        .bind(&i)
                                        .bind(&node.inner.as_ref())
                                        .bind(&(node_index as i64))
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
                    last_id = id.to_owned();
                }
            }
        }
    };

    let (_, _) = tokio::join!(
        server,
        cl_service);
}
