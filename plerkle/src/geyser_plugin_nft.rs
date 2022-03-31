use anchor_client::anchor_lang;
use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

extern crate redis;
use crate::accounts_selector::AccountsSelector;
use crate::error::PlerkleError;
use crate::transaction_selector::TransactionSelector;
use anchor_lang::Event;
use redis::streams::StreamMaxlen;
use redis::Commands;
use redis::{Client, Connection, RedisResult};
use regex::Regex;
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
    ReplicaTransactionInfoVersions, Result, SlotStatus,
};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use {
    log::*,
    solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
    std::{fs::File, io::Read},
    thiserror::Error,
};
use crate::programs::gummy_roll::handle_change_log_event;



#[derive(Default)]
pub struct Plerkle {
    accounts_selector: Option<AccountsSelector>,
    transaction_selector: Option<TransactionSelector>,
    redis_connection: Option<Connection>,
    programs: HashMap<String, Pubkey>
}
impl Plerkle {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_accounts_selector_from_config(config: &serde_json::Value) -> AccountsSelector {
        let accounts_selector = &config["accounts_selector"];

        if accounts_selector.is_null() {
            AccountsSelector::default()
        } else {
            let accounts = &accounts_selector["accounts"];
            let accounts: Vec<String> = if accounts.is_array() {
                accounts
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|val| val.as_str().unwrap().to_string())
                    .collect()
            } else {
                Vec::default()
            };
            let owners = &accounts_selector["owners"];
            let owners: Vec<String> = if owners.is_array() {
                owners
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|val| val.as_str().unwrap().to_string())
                    .collect()
            } else {
                Vec::default()
            };
            AccountsSelector::new(&accounts, &owners)
        }
    }

    fn create_transaction_selector_from_config(config: &serde_json::Value) -> TransactionSelector {
        let transaction_selector = &config["transaction_selector"];

        if transaction_selector.is_null() {
            TransactionSelector::default()
        } else {
            let accounts = &transaction_selector["mentions"];
            let accounts: Vec<String> = if accounts.is_array() {
                accounts
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|val| val.as_str().unwrap().to_string())
                    .collect()
            } else {
                Vec::default()
            };
            TransactionSelector::new(&accounts)
        }
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
        let mut file = File::open(config_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let result: serde_json::Value = serde_json::from_str(&contents).unwrap();
        self.accounts_selector = Some(Self::create_accounts_selector_from_config(&result));
        self.transaction_selector = Some(Self::create_transaction_selector_from_config(&result));
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
        info!("Plugin connected to redis");
        self.programs = [
            (String::from("GR"), Pubkey::from_str("GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD").unwrap()),
            (String::from("GRC"), Pubkey::from_str("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS").unwrap()),
            (String::from("TM"), Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap())
        ].into();
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
                let account_keys = transaction_info.transaction.message().account_keys();

                if transaction_info.transaction_status_meta.status.is_err() {
                    return Ok(());
                }
                return match &self.transaction_selector {
                    Some(transaction_selector)
                    if transaction_selector.is_transaction_selected(
                        transaction_info.is_vote,
                        Box::new(account_keys.iter()),
                    ) =>
                        {
                            let mut keys = account_keys.iter();
                            //TODO -> change this to enums from config
                            let gummy_roll = self.programs.get("GR").unwrap();
                            let match_g = keys.find(|k| {
                                *k == gummy_roll
                            });
                            if match_g.is_some() {
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
                            Ok(())
                        }
                    _ => {
                        Ok(())
                    }
                }
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
