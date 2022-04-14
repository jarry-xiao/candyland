use anchor_client::anchor_lang;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::ops::Index;
use std::result::Iter;

extern crate redis;

use crate::error::PlerkleError;
use anchor_lang::Event;
use redis::streams::StreamMaxlen;
use redis::Commands;
use redis::{Client, Connection, RedisResult};
use regex::Regex;
use solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions, ReplicaTransactionInfo, ReplicaTransactionInfoVersions, Result, SlotStatus};
use std::str::FromStr;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::AccountKeys;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{keccak, pubkeys};
use solana_sdk::transaction::Transaction;
use anchor_client::anchor_lang::AnchorDeserialize;
use hex;
use {
    log::*,
    solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
    std::{fs::File, io::Read},
    thiserror::Error,
};
use gummyroll_crud::InstructionName;
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

#[derive(Default)]
pub struct Plerkle {
    redis_connection: Option<Connection>,
}

impl Plerkle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn txn_contains_program<'a>(keys: AccountKeys, program: &Pubkey) -> bool {
        keys.iter().find(|p| {
            let d = *p;
            d.eq(program)
        }).is_some()
    }

    pub fn order_instructions(
        transaction_info: &ReplicaTransactionInfo,
    ) -> Vec<(Pubkey, CompiledInstruction)> {
        let inner_ixs = transaction_info
            .transaction_status_meta
            .clone()
            .inner_instructions;
        let outer_instructions = transaction_info.transaction.message().instructions();
        let keys = transaction_info.transaction.message().account_keys();
        let mut ordered_ixs: Vec<(Pubkey, CompiledInstruction)> = vec![];
        if inner_ixs.is_some() {
            let inner_ix_list = inner_ixs.as_ref().unwrap().as_slice();
            for inner in inner_ix_list {
                let outer = outer_instructions.get(inner.index as usize).unwrap();
                let program_id = keys.index(outer.program_id_index as usize);
                ordered_ixs.push((*program_id, outer.to_owned()));
                for inner_ix_instance in &inner.instructions {
                    let inner_program_id = keys.index(inner_ix_instance.program_id_index as usize);
                    ordered_ixs.push((*inner_program_id, inner_ix_instance.to_owned()));
                }
            }
        } else {
            for instruction in outer_instructions {
                let program_id = keys.index(instruction.program_id_index as usize);
                ordered_ixs.push((*program_id, instruction.to_owned()));
            }
        }
        ordered_ixs.to_owned()
    }
}

impl Debug for Plerkle {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl GeyserPlugin for Plerkle {
    fn name(&self) -> &'static str {
        "Plerkle"
    }

    fn on_load(&mut self, config_file: &str) -> Result<()> {
        solana_logger::setup_with_default("info");
        info!(
            "Loading plugin {:?} from config_file {:?}",
            self.name(),
            config_file
        );
        let client = redis::Client::open("redis://redis/").unwrap();
        self.redis_connection = client
            .get_connection()
            .map_err(|e| {
                error!("{}", e.to_string());
                GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                    msg: e.to_string(),
                }))
            })
            .ok();
        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Unloading plugin: {:?}", self.name());
    }

    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        _is_startup: bool,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        let ReplicaAccountInfoVersions::V0_0_1(accountv1) = account;
        Ok(())
    }

    fn notify_end_of_startup(
        &mut self,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        Ok(())
    }

    fn update_slot_status(
        &mut self,
        _slot: u64,
        _parent: Option<u64>,
        _status: SlotStatus,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        Ok(())
    }

    fn notify_transaction(
        &mut self,
        transaction: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match transaction {
            ReplicaTransactionInfoVersions::V0_0_1(transaction_info) => {
                if transaction_info.is_vote || transaction_info.transaction_status_meta.status.is_err() {
                    return Ok(());
                }
                // Handle Log PArsing
                let keys = transaction_info.transaction.message().account_keys();
                if keys.iter().any(|v| v == &program_ids::GummyRoll()) {
                    let maxlen = StreamMaxlen::Approx(55000);
                    let change_log_event = handle_change_log_event(transaction_info);
                    if change_log_event.is_ok() {
                        change_log_event.unwrap().iter().for_each(|ev| {
                            let res: RedisResult<()> = self
                                .redis_connection
                                .as_mut()
                                .unwrap()
                                .xadd_maxlen("GM_CL", maxlen, "*", &[("data", ev)]);
                            if res.is_err() {
                                error!("{}", res.err().unwrap());
                            } else {
                                info!("Data Sent")
                            }
                        });
                    }
                }
                /// Handle Instruction Parsing
                let instructions = Plerkle::order_instructions(transaction_info);

                for program_instruction in instructions {
                    match program_instruction {
                        (program, instruction) if program == program_ids::GummyRollCrud() => {
                            let maxlen = StreamMaxlen::Approx(5000);
                            let message = match gummyroll_crud::get_instruction_type(&instruction.data) {
                                gummyroll_crud::InstructionName::Add => {
                                    let data  = instruction.data[8..].to_owned();
                                    let data_buf = &mut data.as_slice();
                                    let add: gummyroll_crud::instruction::Add = gummyroll_crud::instruction::Add::deserialize(data_buf).unwrap();
                                    let tree_id = keys.index(instruction.accounts[3] as usize);
                                    let owner = keys.index(instruction.accounts[0] as usize);
                                    let hex_message = hex::encode(&add.message);
                                    let leaf = keccak::hashv(&[&owner.to_bytes(), add.message.as_slice()]);
                                    let res: RedisResult<()> = self
                                        .redis_connection
                                        .as_mut()
                                        .unwrap()
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
                                    let res: RedisResult<()> = self
                                        .redis_connection
                                        .as_mut()
                                        .unwrap()
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
                                    let leaf = hex::encode(remove.leaf_hash);
                                    let res: RedisResult<()> = self
                                        .redis_connection
                                        .as_mut()
                                        .unwrap()
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
        }
    }


    fn notify_block_metadata(
        &mut self,
        blockinfo: ReplicaBlockInfoVersions,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match blockinfo {
            ReplicaBlockInfoVersions::V0_0_1(block) => {
                info!("Updating block: {:?}", block);
                // block.slot
            }
        }
        Ok(())
    }

    fn account_data_notifications_enabled(&self) -> bool {
        true
    }

    fn transaction_notifications_enabled(&self) -> bool {
        true
    }
}

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
