use hyper::{Body, Request, Response, Server, StatusCode};
// Import the routerify prelude traits.
use futures_util::future::{join3};
use redis::streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply};
use redis::{Commands, Value};
use routerify::prelude::*;
use routerify::{Middleware, Router, RouterService};
use std::{net::SocketAddr};
use std::ops::Index;
use std::str::FromStr;
use anchor_client::solana_sdk::pubkey::Pubkey;
use routerify_json_response::{json_failed_resp_with_message, json_success_resp};
use sea_orm::{DatabaseConnection, DbBackend, SqlxPostgresConnector};
use gummyroll::{ChangeLogEvent, PathNode};
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
use models::{app_specific, cl_items};
use models::prelude::ClItems;
use crate::cl_items::Model;

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

async fn handle_get_tree(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &DatabaseConnection = req.data::<DatabaseConnection>().unwrap();
    let tree_id = hex::decode(req.param("tree_id").unwrap()).unwrap();
    let res = ClItems::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"select distinct on (node_idx) * from cl_items where tree = $1 order by node_idx, seq, level desc;"#,
            vec![tree_id.into()],
        ))
        .all(db)
        .await;
    if res.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, res.err().unwrap().to_string());
    }
    let items: Vec<cl_items::Model> = res.unwrap();

    json_success_resp(&items)
}

async fn handle_get_proof(req: Request<Body>) -> Result<Response<Body>, routerify_json_response::Error> {
    let db: &DatabaseConnection = req.data::<DatabaseConnection>().unwrap();
    let tree_id = decode_tree_id(req.param("tree_id").unwrap()).unwrap();
    let tree = tree_id.clone();
    let index = req.param("index").unwrap().parse::<i64>().unwrap();
    let res = ClItems::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
                    with node as (select level, node_idx from cl_items where node_idx = $1 AND tree = $2 order by seq desc limit 1)
                    select distinct on (c.node_idx) c.hash, c.node_idx, c.level, max(c.seq) as seq, c.id, c.tree from cl_items as c, node as n where c.tree = $2 AND c.level > n.level
                    group by c.hash, c.node_idx, c.level, c.id, c.tree
                    order by c.node_idx, c.level desc
                "#,
            vec![index.into(), tree_id.into()],
        ))
        .all(db)
        .await;


    if res.is_err() {
        return json_failed_resp_with_message(StatusCode::INTERNAL_SERVER_ERROR, res.err().unwrap().to_string());
    }
    let items: Vec<cl_items::Model> = res.unwrap();
    let mut proof: Vec<cl_items::Model> = vec![];
    if items.len() > 0 {
        let mut lvl = 0;
        let nodes = &items;
        let depth = nodes[0].level;
        let mut current_level = 0;
        for (index, node) in nodes.iter().rev().enumerate() {
            if lvl >= depth {
                break;
            }
            if node.level <=  current_level || index == 0{
                continue
            }
            println!("{:?}  {:?}", index, node);
            let mut new_node: Option<&cl_items::Model> = None;
            if node.node_idx % 2 == 0 {
                println!("even \t{:?}  {:?}", nodes.len() - (index + 1), nodes.len());
                new_node = nodes.get(nodes.len() - (index + 1));
                if new_node.is_some() {
                    let can = new_node.unwrap();
                    println!("\t{:?}  {:?}", can.node_idx, can.level);
                    proof.push(can.to_owned())
                } else {
                    proof.push(make_empty_node(tree.to_owned(), lvl, (node.node_idx + 1) as i64))
                }
            } else {
                println!("odd \t{:?}  {:?}", nodes.len() - (index + 1), nodes.len());
                new_node = nodes.get(nodes.len() - (index - 1));
                if new_node.is_some() {
                    let can = new_node.unwrap();
                    println!("\t{:?}  {:?}", can.node_idx, can.level);
                    proof.push(can.to_owned())
                } else {
                    proof.push(make_empty_node(tree.to_owned(), lvl, (node.node_idx - 1) as i64))
                }
            }
            current_level = new_node.unwrap().level + 1;
        }
        proof.push(nodes[0].to_owned());
    }
    json_success_resp(&proof)
}

fn decode_tree_id(param: &String) -> Result<Vec<u8>, ApiError> {
    let pub_key = Pubkey::from_str(&*param).map_err(|e| {
        println!("{}", e.to_string());
        ApiError::ParameterInvalid
    })?;
    Ok(pub_key.to_bytes().to_vec())
}

fn make_empty_node(tree_id: Vec<u8>, lvl: i64, node_index: i64) -> cl_items::Model {
    let mut data = vec![0; 32];
    data.fill_with(|| 0);
    cl_items::Model {
        id: 0,
        tree: tree_id,
        node_idx: node_index,
        seq: 0,
        level: lvl,
        hash: data,
    }
}

fn router(db: DatabaseConnection) -> Router<Body, routerify_json_response::Error> {
    Router::builder()
        .middleware(Middleware::pre(logger))
        .data(db)
        .get("/owner/:account/assets", handle_get_assets)
        .get("/tree/:tree_id", handle_get_tree)
        .get("/assets/:tree_id/:index/proof", handle_get_proof)
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
    let router = router(orm_conn);
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
