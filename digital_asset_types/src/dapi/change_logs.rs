use sea_orm::{DatabaseConnection, DbBackend};
use std::fmt::format;
use sea_orm::sea_query::{Expr, PostgresQueryBuilder};
use {
    crate::dao::asset,
    crate::dao::cl_items,
    sea_orm::{entity::*, query::*, DbErr},
    concurrent_merkle_tree::utils::empty_node,
    crate::rpc::AssetProof,
};

pub async fn get_proof_for_asset(
    db: &DatabaseConnection,
    asset_id: Vec<u8>,
) -> Result<AssetProof, DbErr> {
    let leaf: Option<cl_items::Model> = cl_items::Entity::find()
        .join_rev(
            JoinType::InnerJoin,
            asset::Entity::belongs_to(cl_items::Entity)
                .from(asset::Column::Leaf)
                .to(cl_items::Column::Hash)
                .into(),
        )
        .order_by_asc(cl_items::Column::Level)
        .filter(Expr::cust_with_values("asset.id = ?::bytea", vec![asset_id]))
        .one(db).await?;
    if leaf.is_none() {
        return Err(DbErr::RecordNotFound("Asset Proof Not Found".to_string()));
    }
    let leaf = leaf.unwrap();
    let req_indexes = get_required_nodes_for_proof(leaf.node_idx);
    let expected_proof_size = req_indexes.len();
    let mut final_node_list: Vec<cl_items::Model> = vec![make_default_node(); expected_proof_size];
    let nodesq = cl_items::Entity::find()
        .filter(cl_items::Column::NodeIdx.is_in(req_indexes.clone()))
        .filter(cl_items::Column::Tree.eq(leaf.tree.clone()))
        .order_by_asc(cl_items::Column::Level);
    println!("{:?}", req_indexes.clone());
    println!("{:?}", nodesq.clone().build(DbBackend::Postgres).sql);
    let nodes: Vec<cl_items::Model> = nodesq.all(db).await?;
    println!("{:?}", nodes);

    for node in nodes.iter() {
        if node.level < final_node_list.len().try_into().unwrap() {
            final_node_list[node.level as usize] = node.to_owned();
        }
    }
    if nodes.len() != expected_proof_size {
        for (i, (n, nin)) in final_node_list.iter_mut().zip(req_indexes).enumerate() {
            if *n == make_default_node() {
                *n = make_empty_node(leaf.tree.clone(),i as i64, nin);
            }
        }
    }
    final_node_list[0] = leaf.clone();

    Ok(AssetProof {
        root: bs58::encode(final_node_list.pop().unwrap().hash).into_string(),
        proof: final_node_list
            .iter()
            .map(|model| {
                let node = bs58::encode(&model.hash).into_string();
                println!("{} {} {} {} {}", model.level, model.node_idx, model.seq, bs58::encode(&model.tree).into_string(), node);
                node
            })
            .collect(),
        node_index: leaf.node_idx,
        tree_id: bs58::encode(&leaf.tree).into_string(),
    })
}

fn make_default_node() -> cl_items::Model {
    cl_items::Model {
        id: 0,
        tree: vec![],
        node_idx: 0,
        level: 0,
        hash: vec![],
        seq: 0,
    }
}

fn make_empty_node(tree: Vec<u8>, lvl: i64, node_index: i64) -> cl_items::Model {
    cl_items::Model {
        id: 0,
        tree,
        node_idx: node_index,
        level: lvl,
        hash: empty_node(lvl as u32).to_vec(),
        seq: 0,
    }
}

pub fn get_required_nodes_for_proof(index: i64) -> Vec<i64> {
    let mut indexes = vec![];
    let mut idx = index;
    while idx > 1 {
        if idx % 2 == 0 {
            indexes.push(idx + 1)
        } else {
            indexes.push(idx - 1)
        }
        idx >>= 1
    }
    indexes.push(1);
    return indexes;
}
