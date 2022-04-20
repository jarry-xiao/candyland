use anchor_client::anchor_lang;
use std::ops::Index;

extern crate redis;

use crate::error::PlerkleError;
use crate::macros::*;
use redis::streams::StreamMaxlen;
use redis::Commands;
use redis::{Connection, RedisResult};
use solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions, ReplicaTransactionInfo, ReplicaTransactionInfoVersions, Result, SlotStatus};
use solana_sdk::instruction::CompiledInstruction;
use anchor_client::anchor_lang::AnchorDeserialize;
use solana_sdk::{keccak, pubkeys};
use {
    log::*,
    solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
};
//use gummyroll_crud::InstructionName;
use crate::programs::gummy_roll::handle_change_log_event;

mod program_ids {
    #![allow(missing_docs)]

    use solana_sdk::pubkeys;
    pubkeys!(TokenMetadata, "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");
    pubkeys!(GummyRollCrud, "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");
    pubkeys!(GummyRoll, "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");
    pubkeys!(Token, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    pubkeys!(AToken, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}

pub fn handle_transaction_info(
    transaction_info: &ReplicaTransactionInfo,
    redis_connection: &mut Connection
) -> Result<()> {

    if transaction_info.is_vote || transaction_info.transaction_status_meta.status.is_err() {
        return Ok(());
    }
    // Handle Log PArsing
    let keys = transaction_info.transaction.message().account_keys();
    if keys.iter().any(|v| v == &program_ids::GummyRoll()) {
        let maxlen = StreamMaxlen::Approx(55000);
        let change_log_event = handle_change_log_event(&transaction_info);
        if change_log_event.is_ok() {
            change_log_event.unwrap().iter().for_each(|ev| {
                let res: RedisResult<()> = redis_connection
                    //.as_mut()
                    //.unwrap()
                    .xadd_maxlen("GM_CL", maxlen, "*", &[("data", ev)]);
                if res.is_err() {
                    error!("{}", res.err().unwrap());
                } else {
                    info!("Data Sent")
                }
            });
        }
    }
    // Handle Instruction Parsing
    let instructions = Plerkle::order_instructions(&transaction_info);

    for program_instruction in instructions {
        match program_instruction {
            (program, instruction) if program == program_ids::GummyRollCrud() => {
                let maxlen = StreamMaxlen::Approx(5000);
                let message = match gummyroll_crud::get_instruction_type(&instruction.data) {
                    gummyroll_crud::InstructionName::CreateTree => {
                        warn!("yo yo yo yo");
                        let tree_id = keys.index(instruction.accounts[3] as usize);
                        let auth = keys.index(instruction.accounts[0] as usize);
                        let res: RedisResult<()> = redis_connection
                            //.as_mut()
                            //.unwrap()
                            .xadd_maxlen("GMC_OP", maxlen, "*", &[("op", "create"), ("tree_id", &*tree_id.to_string()), ("authority", &*auth.to_string()) ]);
                        if res.is_err() {
                            error!("{}", res.err().unwrap());
                        } else {
                            info!("Data Sent")
                        }
                    }
                    gummyroll_crud::InstructionName::Add => {
                        let data  = instruction.data[8..].to_owned();
                        let data_buf = &mut data.as_slice();
                        let add: gummyroll_crud::instruction::Add = gummyroll_crud::instruction::Add::deserialize(data_buf).unwrap();
                        let tree_id = keys.index(instruction.accounts[3] as usize);
                        let owner = keys.index(instruction.accounts[0] as usize);
                        let hex_message = hex::encode(&add.message);
                        let leaf = keccak::hashv(&[&owner.to_bytes(), add.message.as_slice()]);
                        let res: RedisResult<()> = redis_connection
                            //.as_mut()
                            //.unwrap()
                            .xadd_maxlen("GMC_OP", maxlen, "*", &[("op", "add"), ("tree_id", &*tree_id.to_string()) , ("leaf", &*leaf.to_string()), ("msg", &*hex_message), ("owner", &*owner.to_string()) ]);
                        if res.is_err() {
                            error!("{}", res.err().unwrap());
                        } else {
                            info!("Data Sent")
                        }
                    },
                    gummyroll_crud::InstructionName::Transfer => {
                        let data  = instruction.data[8..].to_owned();
                        let data_buf = &mut data.as_slice();
                        let add: gummyroll_crud::instruction::Transfer = gummyroll_crud::instruction::Transfer::deserialize(data_buf).unwrap();
                        let tree_id = keys.index(instruction.accounts[3] as usize);
                        let owner = keys.index(instruction.accounts[4] as usize);
                        let new_owner = keys.index(instruction.accounts[5] as usize);
                        let hex_message = hex::encode(&add.message);
                        let leaf = keccak::hashv(&[&new_owner.to_bytes(), add.message.as_slice()]);
                        let res: RedisResult<()> = redis_connection
                            //.as_mut()
                            //.unwrap()
                            .xadd_maxlen("GMC_OP", maxlen, "*", &[("op", "tran"), ("tree_id", &*tree_id.to_string()) , ("leaf", &*leaf.to_string()), ("msg", &*hex_message), ("owner", &*owner.to_string()), ("new_owner", &*new_owner.to_string()) ]);
                        if res.is_err() {
                            error!("{}", res.err().unwrap());
                        } else {
                            info!("Data Sent")
                        }
                    }
                    gummyroll_crud::InstructionName::Remove => {
                        let data  = instruction.data[8..].to_owned();
                        let data_buf = &mut data.as_slice();
                        let remove: gummyroll_crud::instruction::Remove = gummyroll_crud::instruction::Remove::deserialize(data_buf).unwrap();
                        let tree_id = keys.index(instruction.accounts[3] as usize);
                        let owner = keys.index(instruction.accounts[0] as usize);
                        let leaf = bs58::encode(&remove.leaf_hash).into_string();
                        let res: RedisResult<()> = redis_connection
                            //.unwrap()
                            .xadd_maxlen("GMC_OP", maxlen, "*", &[("op", "rm"), ("tree_id", &*tree_id.to_string()) , ("leaf", &*leaf.to_string()), ("msg", ""), ("owner", &*owner.to_string()) ]);
                        if res.is_err() {
                            error!("{}", res.err().unwrap());
                        } else {
                            info!("Data Sent")
                        }
                    }
                    _ => {}
                };

            }
            _ => {}
        };
    }
    Ok(())
}

define_redis_plugin!(
    Plerkle,
    "plerkle",
    handle_transaction_info 
);

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
///
/// This function returns the GeyserPluginPostgres pointer as trait GeyserPlugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = Plerkle::new();
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}

/*
`put uc_merkley on chain
decide hashing parameters and spec
consider new storage design given new uc_merkley


 */
