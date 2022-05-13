use {
    crate::{
        accounts_selector::AccountsSelector,
        error::PlerkleError,
        redis_messenger::RedisMessenger,
        serializer::{
            serialize_account, serialize_block, serialize_slot_status, serialize_transaction,
        },
        transaction_selector::TransactionSelector,
    },
    flatbuffers::FlatBufferBuilder,
    log::*,
    messenger::{Messenger, ACCOUNT_STREAM, BLOCK_STREAM, SLOT_STREAM, TRANSACTION_STREAM},
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPlugin, GeyserPluginError, ReplicaAccountInfoVersions, ReplicaBlockInfoVersions,
        ReplicaTransactionInfoVersions, Result, SlotStatus,
    },
    solana_sdk::{message::AccountKeys, pubkey::Pubkey},
    std::{
        fmt::{Debug, Formatter},
        fs::File,
        io::Read,
    },
};

#[derive(Default)]
pub struct Plerkle<T: Messenger + Default> {
    accounts_selector: Option<AccountsSelector>,
    transaction_selector: Option<TransactionSelector>,
    messenger: Option<T>,
}

impl<T: Messenger + Default> Plerkle<T> {
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

    // Currently not used but may want later.
    pub fn _txn_contains_program<'b>(keys: AccountKeys, program: &Pubkey) -> bool {
        keys.iter()
            .find(|p| {
                let d = *p;
                d.eq(program)
            })
            .is_some()
    }
}

impl<T: Messenger + Default> Debug for Plerkle<T> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl<T: 'static + Messenger + Default + Send + Sync> GeyserPlugin for Plerkle<T> {
    fn name(&self) -> &'static str {
        "Plerkle"
    }

    fn on_load(&mut self, config_file: &str) -> Result<()> {
        solana_logger::setup_with_default("info");

        // Read in config file.
        info!(
            "Loading plugin {:?} from config_file {:?}",
            self.name(),
            config_file
        );
        let mut file = File::open(config_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Setup accounts and transaction selectors based on config file JSON.
        let result = serde_json::from_str(&contents);
        match result {
            Ok(config) => {
                self.accounts_selector = Some(Self::create_accounts_selector_from_config(&config));
                self.transaction_selector =
                    Some(Self::create_transaction_selector_from_config(&config));
            }
            Err(err) => {
                return Err(GeyserPluginError::ConfigFileReadError {
                    msg: format!("Could not read config file JSON: {:?}", err),
                })
            }
        }

        // Setup messenger.
        let mut messenger = T::new()?;
        messenger.add_stream(ACCOUNT_STREAM);
        messenger.add_stream(SLOT_STREAM);
        messenger.add_stream(TRANSACTION_STREAM);
        messenger.add_stream(BLOCK_STREAM);
        messenger.set_buffer_size(ACCOUNT_STREAM, 5000);
        messenger.set_buffer_size(SLOT_STREAM, 5000);
        messenger.set_buffer_size(TRANSACTION_STREAM, 5000);
        messenger.set_buffer_size(BLOCK_STREAM, 5000);
        self.messenger = Some(messenger);

        Ok(())
    }

    fn on_unload(&mut self) {
        info!("Unloading plugin: {:?}", self.name());
    }

    fn update_account(
        &mut self,
        account: ReplicaAccountInfoVersions,
        slot: u64,
        is_startup: bool,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match account {
            ReplicaAccountInfoVersions::V0_0_1(account) => {
                // Check if account was selected in config.
                if let Some(accounts_selector) = &self.accounts_selector {
                    if !accounts_selector.is_account_selected(account.pubkey, account.owner) {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }

                // Send account info over messenger.
                match &mut self.messenger {
                    None => Err(GeyserPluginError::Custom(Box::new(
                        PlerkleError::DataStoreConnectionError {
                            msg: "There is no connection to data store.".to_string(),
                        },
                    ))),
                    Some(messenger) => {
                        let mut builder = FlatBufferBuilder::new();
                        let bytes = serialize_account(&mut builder, account, slot, is_startup);
                        messenger.send(ACCOUNT_STREAM, bytes)
                    }
                }
            }
        }
    }

    fn notify_end_of_startup(
        &mut self,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        Ok(())
    }

    fn update_slot_status(
        &mut self,
        slot: u64,
        parent: Option<u64>,
        status: SlotStatus,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        // Send slot status over messenger.
        match &mut self.messenger {
            None => Err(GeyserPluginError::Custom(Box::new(
                PlerkleError::DataStoreConnectionError {
                    msg: "There is no connection to data store.".to_string(),
                },
            ))),
            Some(messenger) => {
                let mut builder = FlatBufferBuilder::new();
                let bytes = serialize_slot_status(&mut builder, slot, parent, status);
                messenger.send(SLOT_STREAM, bytes)
            }
        }
    }

    fn notify_transaction(
        &mut self,
        transaction_info: ReplicaTransactionInfoVersions,
        slot: u64,
    ) -> solana_geyser_plugin_interface::geyser_plugin_interface::Result<()> {
        match transaction_info {
            ReplicaTransactionInfoVersions::V0_0_1(transaction_info) => {
                // Don't log votes or transactions with error status.
                if transaction_info.is_vote
                    || transaction_info.transaction_status_meta.status.is_err()
                {
                    return Ok(());
                }

                // Check if transaction was selected in config.
                if let Some(transaction_selector) = &self.transaction_selector {
                    if !transaction_selector.is_transaction_selected(
                        transaction_info.is_vote,
                        Box::new(transaction_info.transaction.message().account_keys().iter()),
                    ) {
                        return Ok(());
                    }
                } else {
                    return Ok(());
                }

                // Send transaction info over messenger.
                match &mut self.messenger {
                    None => Err(GeyserPluginError::Custom(Box::new(
                        PlerkleError::DataStoreConnectionError {
                            msg: "There is no connection to data store.".to_string(),
                        },
                    ))),
                    Some(messenger) => {
                        let mut builder = FlatBufferBuilder::new();
                        let bytes = serialize_transaction(&mut builder, transaction_info, slot);
                        messenger.send(TRANSACTION_STREAM, bytes)
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
            ReplicaBlockInfoVersions::V0_0_1(block_info) => {
                info!("Updating block: {:?}", block_info);

                // Send block info over messenger.
                match &mut self.messenger {
                    None => Err(GeyserPluginError::Custom(Box::new(
                        PlerkleError::DataStoreConnectionError {
                            msg: "There is no connection to data store.".to_string(),
                        },
                    ))),
                    Some(messenger) => {
                        let mut builder = FlatBufferBuilder::new();
                        let bytes = serialize_block(&mut builder, block_info);
                        messenger.send(BLOCK_STREAM, bytes)
                    }
                }
            }
        }
    }

    fn account_data_notifications_enabled(&self) -> bool {
        self.accounts_selector
            .as_ref()
            .map_or_else(|| false, |selector| selector.is_enabled())
    }

    fn transaction_notifications_enabled(&self) -> bool {
        self.transaction_selector
            .as_ref()
            .map_or_else(|| false, |selector| selector.is_enabled())
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
/// # Safety
///
/// This function returns the GeyserPluginPostgres pointer as trait GeyserPlugin.
pub unsafe extern "C" fn _create_plugin() -> *mut dyn GeyserPlugin {
    let plugin = Plerkle::<RedisMessenger>::new();
    let plugin: Box<dyn GeyserPlugin> = Box::new(plugin);
    Box::into_raw(plugin)
}
