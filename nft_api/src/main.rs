use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::join;
use merk::Merk;
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{AsyncCommands, Value};
use routerify::prelude::*;
use routerify::{Middleware, RequestInfo, Router, RouterService};
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};
use std::sync::Mutex;
use std::{convert::Infallible, net::SocketAddr};
use crate::events::handle_event;

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
        let mut conn = client.clone().get_async_connection().await.unwrap();
        // TODO -> save last id in persistent for restart
        // TODO -> dedup buffer
        let opts = StreamReadOptions::default();
        let srr: StreamReadReply = conn.xread_options(&["GM_CL"], &["$"], &opts).await.unwrap();
        for StreamKey { key, ids } in srr.keys {
            println!("Stream {}", key);
            for StreamId { id, map } in ids {
                println!("\tID {}", id);
                for (n, s) in map {
                    if let Value::Data(bytes) = s {
                        let raw_str = String::from_utf8(bytes);
                        match raw_str {
                            Ok(data) => {
                                println!("{}", &data);
                            }
                            _ =>{
                                eprintln!("Base64 error")
                            }
                        }


                    } else {
                        panic!("Weird data")
                    }
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
