
use std::fmt::{Debug, Formatter};
extern crate redis;
use redis::{Client, Connection};
use redis::Commands;
use crate::accounts_selector::AccountsSelector;
use crate::transaction_selector::TransactionSelector;
use solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions, ReplicaTransactionInfoVersions, Result, SlotStatus};
use {
    log::*,
    solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPlugin,
    std::{fs::File, io::Read},
    thiserror::Error,
};

#[derive(Default)]
pub struct Plerkle {
    accounts_selector: Option<AccountsSelector>,
    transaction_selector: Option<TransactionSelector>,
    redis_connection: Option<Connection>,
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

#[derive(Error, Debug)]
pub enum PlerkleError {
    #[error("Error connecting to the backend data store. Error message: ({msg})")]
    DataStoreConnectionError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    DataSchemaError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    ConfigurationError { msg: String },
}

impl Debug for Plerkle {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
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

        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        self.redis_connection = client
            .get_connection()
            .map_err(|e| GeyserPluginError::Custom(
                Box::new(PlerkleError::ConfigurationError { msg: e.to_string() })
            ))
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
        info!("Updating account: {:?} {:?}", accountv1, slot);
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
            ReplicaTransactionInfoVersions::V0_0_1(transactionv1) => {
                info!("Updating txn: {:?} {:?}", transactionv1, slot);
                let transaction_info = transactionv1;
                if let Some(transaction_selector) = &self.transaction_selector {
                    if !transaction_selector.is_transaction_selected(
                        transaction_info.is_vote,
                        transaction_info.transaction.message().account_keys_iter(),
                    ) {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }
                if transactionv1.transaction_status_meta.status.is_err() {
                    return Ok(());
                }
                let txid = format!("txn.{}", transactionv1.signature.to_string());
                let con  = self.redis_connection.as_mut().unwrap();
                redis::cmd("SET")
                    .arg(&[txid, slot.to_string()])
                    .query(con)
                    .map_err(|e| GeyserPluginError::Custom(
                        Box::new(PlerkleError::ConfigurationError { msg: e.to_string() })
                    ))?;
            }
        }
        Ok(())
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
