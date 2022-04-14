use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService};
use std::{net::SocketAddr};
use std::borrow::Borrow;
use std::ops::Index;
use std::str::FromStr;
use anchor_client::solana_sdk::pubkey::Pubkey;
use futures_util::StreamExt;
use routerify_json_response::{json_failed_resp, json_failed_resp_with_message, json_success_resp};
use gummyroll::{ChangeLogEvent, empty_node, PathNode};
use sqlx;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use serde::{Serialize, Deserialize};

mod events;
mod error;

use events::handle_event;
use error::ApiError;
use tokio::{join, task};

async fn logger(req: Request<Body>) -> Result<Request<Body>, routerify_json_response::Error> {
    println!(
        "{} {} {}",
        req.remote_addr(),
        req.method(),
        req.uri().path()
    );
    Ok(req)
}

// async fn handle_get_assets(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
//     let db: &Pool<Postgres> = req.data::<&Pool<Postgres>>().unwrap();
//     let owner = req.param("account").unwrap();
//     let res = AppSpecific::find()
//         .filter(app_specific::Column::Owner.eq(owner.to_owned()))
//         .all(db)
//         .await;
//     if res.is_err() {
//         return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, res.err().unwrap().to_string());
//     }
//     json_success_resp(&res.unwrap())
// }

#[derive(sqlx::FromRow, Clone, Debug)]
struct NodeDAO {
    pub hash: Vec<u8>,
    pub level: i64,
    pub node_idx: i64,
    pub seq: i64,
}

#[derive(sqlx::FromRow)]
struct Root {
    pub hash: Vec<u8>,
}


#[derive(Serialize)]
struct NodeView {
    pub hash: String,
    pub level: i64,
    pub index: i64,
}

fn list_to_view(items: Vec<NodeDAO>) -> Vec<NodeView> {
    let mut view = vec![];
    for r in items {
        view.push(to_view(r))
    }
    view
}

fn to_view(r: NodeDAO) -> NodeView {
    NodeView {
        hash: bs58::encode(r.hash).into_string(),
        level: r.level,
        index: r.node_idx,
    }
}

fn encode_root(root: Root) -> String {
    bs58::encode(root.hash).into_string()
}

async fn handler_get_assets_for_owner(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error>  {

    json_success_resp(&String::new())
}

async fn handle_get_tree(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = hex::decode(req.param("tree_id").unwrap()).unwrap();
    let results = sqlx::query_as::<_, NodeDAO>("select distinct on (node_idx), node_index, level, hash, seq from cl_items where tree = $1 order by seq, node_idx, level desc")
        .bind(tree_id)
        .fetch_all(db).await;
    if results.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, results.err().unwrap().to_string());
    }
    json_success_resp(&list_to_view(results.unwrap()))
}

async fn handle_get_root(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_tree_id(req.param("tree_id").unwrap()).unwrap();
    let result = sqlx::query_as::<_, Root>("select hash from cl_items where node_idx = 1 AND tree = $1 order by seq desc limit 1")
        .bind(tree_id)
        .fetch_one(db).await;
    if result.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, result.err().unwrap().to_string());
    }
    json_success_resp(&encode_root(result.unwrap()))
}

async fn handle_get_proof(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_tree_id(req.param("tree_id").unwrap()).unwrap();
    let index = req.param("index").unwrap().parse::<i64>().unwrap();
    let nodes = get_required_nodes_for_proof(index);
    let expected_proof_size = nodes.len();
    let results = sqlx::query_as::<_, NodeDAO>(r#"
    select distinct on (node_idx) node_idx, hash, level, max(seq) as seq
    from cl_items where node_idx = ANY ($1) and tree = $2
    group by seq, node_idx, level, hash
    order by node_idx desc, seq desc
    "#
    )
        .bind(&nodes.as_slice())
        .bind(&tree_id)
        .fetch_all(db).await;
    if results.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, results.err().unwrap().to_string());
    }
    let nodes_from_db = results.unwrap();
    let mut final_node_list: Vec<NodeView> = vec![];
    if nodes_from_db.len() > expected_proof_size {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, "Tree Corrupted");
    }
    let mut searched = 0;
    if nodes_from_db.len() != expected_proof_size {
        let things : Vec<i64> =  nodes_from_db.iter().map(|i| i.node_idx).collect();
        for i in 0..nodes_from_db.len() {
            let returned = nodes_from_db[i].to_owned();
            for j in searched..nodes.len() {
                let expected = nodes[j];
                if returned.node_idx != expected {
                    final_node_list.push(to_view(make_empty_node(searched as i64, expected)));
                    searched = j+1;
                } else {
                    final_node_list.push(to_view(returned));
                    searched = j+1;
                    break;
                }
            }
        }
        for i in searched..nodes.len() {
            let expected = nodes[i];
            final_node_list.push(to_view(make_empty_node(i as i64, expected)));
        }
    }
    json_success_resp(&final_node_list)
}

fn get_required_nodes_for_proof(index: i64) -> Vec<i64> {
    let mut indexes = vec![];
    let mut idx = index;
    while idx >= 1 {
        if idx % 2 == 0 { indexes.push(idx + 1) } else { indexes.push(idx - 1) }
        idx >>= 1
    }
    return indexes;
}

fn decode_tree_id(param: &String) -> Result<Vec<u8>, ApiError> {
    let pub_key = Pubkey::from_str(&*param).map_err(|e| {
        println!("{}", e.to_string());
        ApiError::ParameterInvalid
    })?;
    Ok(pub_key.to_bytes().to_vec())
}

fn make_empty_node(lvl: i64, node_index: i64) -> NodeDAO {
    let mut data = vec![0; 32];
    data.fill_with(|| 0);
    NodeDAO {
        node_idx: node_index,
        level: lvl,
        hash: data,
        seq: 0,
    }
}

fn router(db: Pool<Postgres>) -> Router<Body, routerify_json_response::Error> {
    Router::builder()
        .middleware(Middleware::pre(logger))
        .data(db)
        .get("/owner/:pubkey/assets", handler_get_assets_for_owner)
        .get("/tree/:tree_id", handle_get_tree)
        .get("/root/:tree_id", handle_get_root)
        .get("/proof/:tree_id/:index", handle_get_proof)
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
    let router = router(main_pool);
    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();
    // The address on which the server will be listening.
    let addr = SocketAddr::from(([0, 0, 0, 0], 9090));
    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {}", addr);
    if let Err(err) = server.await {
        eprintln!("Server error: {}", err);
    }
}
