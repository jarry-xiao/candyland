use hyper::{Body, header, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, RequestInfo, Router, RouterService};
use std::{net::SocketAddr};
use std::borrow::Borrow;
use std::convert::Infallible;
use std::ops::Index;
use std::str::FromStr;
use anchor_client::solana_sdk::pubkey::Pubkey;
use futures_util::StreamExt;
use hyper::header::HeaderValue;
use routerify_json_response::{json_failed_resp, json_failed_resp_with_message, json_success_resp};
use gummyroll::{ChangeLogEvent, empty_node, PathNode};
use sqlx;
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use serde::{Serialize, Deserialize};
use std::io;

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

#[derive(sqlx::FromRow, Clone, Debug)]
struct NodeDAO {
    pub hash: Vec<u8>,
    pub level: i64,
    pub node_idx: i64,
    pub seq: i64,
}

#[derive(sqlx::FromRow, Clone, Debug)]
struct AssetDAO {
    pub data: String,
    pub index: i64,
    pub owner: Vec<u8>,
    pub tree: Vec<u8>,
    pub admin: Vec<u8>,
    pub hash: Vec<u8>,
}

#[derive(Serialize)]
struct AssetView {
    pub data: String,
    pub index: i64,
    pub owner: String,
    pub treeAccount: String,
    pub treeAdmin: String,
    pub hash: String,
}

#[derive(sqlx::FromRow)]
struct Root {
    pub hash: Vec<u8>,
}


#[derive(Serialize, Default, Clone, PartialEq)]
struct NodeView {
    pub hash: String,
    pub level: i64,
    pub index: i64,
}

#[derive(Serialize)]
struct AssetProof {
    pub root: String,
    pub hash: String,
    pub proof: Vec<String>,
}

fn node_list_to_view(items: Vec<NodeDAO>) -> Vec<NodeView> {
    let mut view = vec![];
    for r in items {
        view.push(node_to_view(r))
    }
    view
}

fn node_to_view(r: NodeDAO) -> NodeView {
    NodeView {
        hash: bs58::encode(r.hash).into_string(),
        level: r.level,
        index: r.node_idx,
    }
}

fn asset_list_to_view(items: Vec<AssetDAO>) -> Vec<AssetView> {
    let mut view = vec![];
    for r in items {
        view.push(asset_to_view(r))
    }
    view
}


fn asset_to_view(r: AssetDAO) -> AssetView {
    AssetView {
        index: r.index,
        treeAccount: bs58::encode(r.tree).into_string(),
        owner: bs58::encode(r.owner).into_string().to_string(),
        treeAdmin: bs58::encode(r.admin).into_string().to_string(),
        hash: bs58::encode(r.hash).into_string().to_string(),
        data: r.data,
    }
}

fn encode_root(root: Root) -> String {
    bs58::encode(root.hash).into_string()
}


async fn handle_get_asset(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_b58_param(req.param("tree_id").unwrap()).unwrap();
    let index = req.param("index").unwrap().parse::<i64>().unwrap();

    let result = get_asset(db, tree_id, index).await;
    if result.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, result.err().unwrap().to_string());
    }
    let asset = result.unwrap();
    json_success_resp(&asset)
}


async fn handler_get_assets_for_owner(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let owner = decode_b58_param(req.param("owner").unwrap()).unwrap();

    let results = sqlx::query_as::<_, AssetDAO>(r#"
    select a.msg as data, c.node_idx as index, a.owner, a.tree_id as tree , aso.authority as admin, a.leaf as hash, max(seq) as seq from app_specific as a
    join cl_items as c on c.tree = a.tree_id and c.hash = a.leaf
    join app_specific_ownership aso on a.tree_id = aso.tree_id
    where a.owner = $1
    group by c.node_idx, a.msg, a.owner, a.tree_id, aso.authority, a.leaf
    order by seq"#
    )
        .bind(owner)
        .fetch_all(db).await;
    if results.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, results.err().unwrap().to_string());
    }
    let assets = results.unwrap();
    json_success_resp(&asset_list_to_view(assets))
}

async fn handle_get_tree(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_b58_param(req.param("tree_id").unwrap()).unwrap();
    let results = sqlx::query_as::<_, NodeDAO>("select distinct on (node_idx), node_index, level, hash, seq from cl_items where tree = $1 order by seq, node_idx, level desc")
        .bind(tree_id)
        .fetch_all(db).await;
    if results.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, results.err().unwrap().to_string());
    }
    json_success_resp(&node_list_to_view(results.unwrap()))
}

async fn handle_get_root(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_b58_param(req.param("tree_id").unwrap()).unwrap();
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
    let tree_id = decode_b58_param(req.param("tree_id").unwrap()).unwrap();
    let index = req.param("index").unwrap().parse::<i64>().unwrap();


    let proof = get_proof(db, tree_id, index).await;
    if proof.is_err() {
        return if let ApiError::ResponseError { status, msg } = proof.err().unwrap() {
            json_failed_resp_with_message(status, msg)
        } else {
            json_failed_resp(StatusCode::INTERNAL_SERVER_ERROR)
        };
    }
    json_success_resp(&proof.unwrap())
}

async fn handle_get_asset_proof(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &Pool<Postgres> = req.data::<Pool<Postgres>>().unwrap();
    let tree_id = decode_b58_param(req.param("tree_id").unwrap()).unwrap();
    let tree_id2 = tree_id.as_slice().to_vec(); // WHY RUST
    let index = req.param("index").unwrap().parse::<i64>().unwrap();

    let result = get_asset(db, tree_id, index).await;
    let proof: Result<Vec<String>, ApiError> = get_proof(db, tree_id2, index).await.map(|p| {
        p.iter().map(|node| {
            node.hash.clone()
        })
            .collect()
    });
    let asset_proof = result.and_then(|n| {
        proof.map(|p| {
            AssetProof {
                hash: n.hash,
                root: p.last().unwrap().clone(),
                proof: p,
            }
        })
    });

    if asset_proof.is_err() {
        return if let ApiError::ResponseError { status, msg } = asset_proof.err().unwrap() {
            json_failed_resp_with_message(status, msg)
        } else {
            json_failed_resp(StatusCode::INTERNAL_SERVER_ERROR)
        };
    }
    json_success_resp(&asset_proof.unwrap())
}


async fn get_asset(db: &Pool<Postgres>, tree_id: Vec<u8>, index: i64) -> Result<AssetView, ApiError> {
    let result = sqlx::query_as::<_, AssetDAO>(r#"
    select a.msg as data, c.node_idx as index, a.owner, a.tree_id as tree , aso.authority as admin, a.leaf as hash, max(seq) as seq from app_specific as a
            join cl_items as c on c.tree = a.tree_id and c.hash = a.leaf
            join app_specific_ownership aso on a.tree_id = aso.tree_id
    where a.tree_id = $1 AND c.node_idx = $2
    group by c.node_idx, a.msg, a.owner, a.tree_id, aso.authority, a.leaf
    order by seq
    limit 1
    "#
    )
        .bind(&tree_id)
        .bind(&index)
        .fetch_one(db).await;
    result.map(asset_to_view).map_err(|e| {
        return ApiError::ResponseError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: e.to_string(),
        };
    })
}

async fn get_proof(db: &Pool<Postgres>, tree_id: Vec<u8>, index: i64) -> Result<Vec<NodeView>, ApiError> {
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
    let nodes_from_db = results.unwrap();
    let mut final_node_list: Vec<NodeView> = vec![NodeView::default(); expected_proof_size];
    if nodes_from_db.len() > expected_proof_size {
        return Err(ApiError::ResponseError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: "Tree Corrupted".to_string(),
        });
    }
    if nodes_from_db.len() != expected_proof_size {
        for returned in nodes_from_db.iter() {
            final_node_list[returned.level as usize] = node_to_view(returned.to_owned());
        }
        for (i, (n, nin)) in final_node_list.iter_mut().zip(nodes).enumerate() {
            if *n == NodeView::default() {
                *n = node_to_view(make_empty_node(i as i64, nin));
            }
        }
    } else {
        final_node_list = node_list_to_view(nodes_from_db);
    }
    Ok(final_node_list)
}

fn get_required_nodes_for_proof(index: i64) -> Vec<i64> {
    let mut indexes = vec![];
    let mut idx = index;
    while idx > 1 {
        if idx % 2 == 0 { indexes.push(idx + 1) } else { indexes.push(idx - 1) }
        idx >>= 1
    }
    indexes.push(1);
    println!("nodes {:?}", indexes);
    return indexes;
}

fn decode_b58_param(param: &String) -> Result<Vec<u8>, ApiError> {
    let pub_key = Pubkey::from_str(&*param).map_err(|e| {
        println!("{}", e.to_string());
        ApiError::ParameterInvalid
    })?;
    Ok(pub_key.to_bytes().to_vec())
}



fn make_empty_node(lvl: i64, node_index: i64) -> NodeDAO {
    NodeDAO {
        node_idx: node_index,
        level: lvl,
        hash: empty_node(lvl as u32).inner.to_vec(),
        seq: 0,
    }
}

fn router(db: Pool<Postgres>) -> Router<Body, routerify_json_response::Error> {
    Router::builder()
        .middleware(Middleware::pre(logger))
        .middleware(Middleware::post(|mut res| async move {
            let headers = res.headers_mut();
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
            headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("*"));
            headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("*"));
            headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, HeaderValue::from_static("*"));
            Ok(res)
        }))
        .data(db)
        .get("/assets/:tree_id/:index/proof", handle_get_asset_proof)
        .get("/assets/:tree_id/:index", handle_get_asset)
        .get("/owner/:owner/assets", handler_get_assets_for_owner)
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
