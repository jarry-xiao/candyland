use anchor_client::anchor_lang;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::{
    fmt::{Debug, Formatter},
    ops::Index,
    rc::Rc,
    cell::RefCell,
};
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

#[macro_use]
pub mod macros {
    use super::*;

    macro_rules! define_redis_plugin {
        ($struct:ident, $name:literal, $func:ident) => {
            #[derive(Default)]
            pub struct $struct {
                redis_connection: Option<Connection>,
            }

            impl $struct {
                pub fn new() -> Self {
                    Self::default()
                }

                pub fn txn_contains_program<'a>(keys: solana_sdk::message::AccountKeys, program: &solana_sdk::pubkey::Pubkey) -> bool {
                    keys.iter().find(|p| {
                        let d = *p;
                        d.eq(program)
                    }).is_some()
                }

                pub fn order_instructions(
                    transaction_info: &ReplicaTransactionInfo,
                ) -> Vec<(solana_sdk::pubkey::Pubkey, CompiledInstruction)> {
                    let inner_ixs = transaction_info
                        .transaction_status_meta
                        .clone()
                        .inner_instructions;
                    let outer_instructions = transaction_info.transaction.message().instructions();
                    let keys = transaction_info.transaction.message().account_keys();
                    let mut ordered_ixs: Vec<(solana_sdk::pubkey::Pubkey, CompiledInstruction)> = vec![];
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

            impl std::fmt::Debug for $struct {
                fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    Ok(())
                }
            }

            impl GeyserPlugin for $struct {
                fn name(&self) -> &'static str {
                    $name
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
                            self.$func(transaction_info);
                            /*
                            match self.redis_connection {
                                Some(mut connection) => {
                                    $func(transaction_info, &mut connection);
                                }
                                None => {
                                    error!("No redis connection available to parse messages");
                                    GeyserPluginError::Custom(Box::new(PlerkleError::ConnectionNoneError {
                                        msg: "No redis connection available to parse messages".to_string(),
                                    }));
                                }
                            }
                            */
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
        }
    }
}
