use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::join;
use merk::Merk;
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
use gummyroll::ChangeLogEvent;
use crate::events::handle_event;
use tokio_postgres::{NoTls, Error};
use crate::error::ApiError;

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

fn router(merkle_db: Merk) -> Router<Body, routerify_json_response::Error> {
    let a = Mutex::new(merkle_db);
    Router::builder()
        .data(a)
        .middleware(Middleware::pre(logger))
        .get("/assets/:account", api::handle_get_assets)
        /*
        assets: [
        {
        data: "",
        tree_id: "",
        index: "",
        }
        ]
        .get("/tree/:tree_id", api::handle_get_assets)
        .get("/proof/:tree_id/:index", api::handle_get_assets)
        {
            proof: [Pubkeys],
            root: Pubkey
        }
         */
        .build()
        .unwrap()
}

#[tokio::main]
async fn main() {
    let merk = Merk::open("./merk.db").unwrap();
    let (mut psclient,connection) = tokio_postgres::connect("host=db user=solana password=solana", NoTls).await.unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let set_clsql= psclient.prepare("INSERT INTO cl_meta (tree, leaf_idx, seq) VALUES ($1,$2,$3)").await.unwrap();
    let set_clsql_item= psclient.prepare("INSERT INTO cl_items (tree, seq, level, hash) VALUES ($1,$2,$3,$4)").await.unwrap();
    let client = redis::Client::open("redis://redis/").unwrap();
    let router = router(merk);
    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();
    // The address on which the server will be listening.
    let addr = SocketAddr::from(([0, 0, 0, 0], 9090));
    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {}", addr);
    let data_service = tokio::spawn(async move {
        let conn_res = client.get_connection();
        let mut conn = conn_res.unwrap();
        println!("service");
        // TODO -> save last id in persistent for restart
        // TODO -> dedup buffer
        // TODO -> This code is SO bad all MVP
        let opts = StreamReadOptions::default().block(0).count(1000);
        let mut last_id: String = "$".to_string();
        loop {
            let srr: StreamReadReply = conn.xread_options(&["GM_CL"], &[&last_id], &opts).unwrap();
            for StreamKey { key, ids } in srr.keys {
                println!("Stream {}", key);
                for StreamId { id, map } in ids {
                    println!("\tID {}", id);
                    let pid = id.replace("-", "").parse::<i64>().unwrap();
                    for (n, s) in map {
                        if let Value::Data(bytes) = s {
                            let raw_str = String::from_utf8(bytes);
                            match raw_str {
                                Ok(data) => {
                                    let clr: Result<ChangeLogEvent, ApiError> = handle_event(data);
                                    match clr {
                                        Ok(cl) => {
                                            let txnb = psclient.transaction().await;
                                            match txnb {
                                                Ok(txn) => {
                                                    txn.execute(&set_clsql,
                                                        &[&cl.id.as_ref(), &i64::from(cl.index), &pid]
                                                    ).await.unwrap();
                                                    let mut i: i64 = 0;
                                                    for el in cl.path.into_iter() {
                                                        txn.execute(&set_clsql_item,
                                                                            &[&cl.id.as_ref(),
                                                                                &pid,
                                                                                &i,
                                                                                &el.inner.as_ref()]
                                                        ).await.unwrap();
                                                       i+=1;
                                                    }
                                                    match txn.commit().await {
                                                        Ok(r) => {
                                                            println!("Saved CL");
                                                        },
                                                        Err(e) => {
                                                            eprintln!("{}", e.to_string())
                                                        }
                                                    }

                                                },
                                                Err(e) => {
                                                    eprintln!("{}", e.to_string())
                                                }
                                            }
                                        },
                                        Err(e) => {
                                            eprintln!("{}", e.to_string())
                                        }
                                    }
                                }
                                _ => {
                                    eprintln!("Base64 error")
                                }
                            }
                        } else {
                            panic!("Weird data")
                        }
                    }
                    last_id = id.to_owned();
                }
            }
        }
    });
    match join(server, data_service).await {
        (Err(err), _) => {
            eprintln!("Server error: {}", err);
        }
        (_, Err(err)) => {
            eprintln!("Data Service error: {}", err);
        }
        _ => {
            eprintln!("Closing");
        }
    }
}
