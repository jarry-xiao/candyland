use {
    crate::{error::PlerkleError, programs::gummy_roll::handle_change_log_event},
    anchor_client::anchor_lang::AnchorDeserialize,
    log::*,
    redis::{streams::StreamMaxlen, Commands, Connection, RedisResult, ToRedisArgs},
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, ReplicaAccountInfo, ReplicaTransactionInfo, Result,
    },
    solana_sdk::{instruction::CompiledInstruction, keccak, pubkey::Pubkey},
    std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
        ops::Index,
    },
};

mod program_ids {
    #![allow(missing_docs)]

    use solana_sdk::pubkeys;
    pubkeys!(
        token_metadata,
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    );
    pubkeys!(
        gummy_roll_crud,
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
    );
    pubkeys!(gummy_roll, "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");
    pubkeys!(token, "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    pubkeys!(a_token, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn send_account(&self, account: &ReplicaAccountInfo) -> Result<()>;
    fn send_transaction(&mut self, transaction_info: &ReplicaTransactionInfo) -> Result<()>;
    fn recv_account(&self) -> Result<()>;
    fn recv_transaction(&self) -> Result<()>;
}

#[derive(Default)]
pub struct RedisMessenger {
    connection: Option<Connection>,
    streams: HashMap<String, RedisMessengerStream>,
}

pub struct RedisMessengerStream {
    buffer_size: StreamMaxlen,
    name: String,
}

impl RedisMessenger {
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

    pub fn _add_stream(&mut self, name: String, max_buffer_size: usize) {
        self.streams.insert(
            name.clone(),
            RedisMessengerStream {
                name,
                buffer_size: StreamMaxlen::Approx(max_buffer_size),
            },
        );
    }

    pub fn _get_stream(&self, name: String) -> Option<&RedisMessengerStream> {
        self.streams.get(&*name)
    }

    pub fn _add<K: ToRedisArgs, T: ToRedisArgs>(
        &mut self,
        stream: RedisMessengerStream,
        id: String,
        items: &[(K, T)],
    ) -> Result<()> {
        let conn = self.connection.as_mut().unwrap();
        conn.xadd_maxlen(stream.name, stream.buffer_size, &*id, items)
            .map_err(|e| {
                GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                    msg: e.to_string(),
                }))
            })
    }
}

impl Messenger for RedisMessenger {
    fn new() -> Result<Self> {
        // Setup Redis client.
        let client = redis::Client::open("redis://redis/").unwrap();
        let connection = client.get_connection().map_err(|e| {
            error!("{}", e.to_string());
            GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                msg: e.to_string(),
            }))
        })?;

        Ok(Self {
            connection: Some(connection),
            streams: HashMap::<String, RedisMessengerStream>::default(),
        })
    }

    fn send_account(&self, _account: &ReplicaAccountInfo) -> Result<()> {
        Ok(())
    }

    fn send_transaction(&mut self, transaction_info: &ReplicaTransactionInfo) -> Result<()> {
        // Handle log parsing.
        let keys = transaction_info.transaction.message().account_keys();
        if keys.iter().any(|v| v == &program_ids::gummy_roll()) {
            let maxlen = StreamMaxlen::Approx(55000);
            let change_log_event = handle_change_log_event(&transaction_info);
            if change_log_event.is_ok() {
                change_log_event.unwrap().iter().for_each(|ev| {
                    let res: RedisResult<()> = self.connection.as_mut().unwrap().xadd_maxlen(
                        "GM_CL",
                        maxlen,
                        "*",
                        &[("data", ev)],
                    );
                    if res.is_err() {
                        error!("{}", res.err().unwrap());
                    } else {
                        info!("Data Sent")
                    }
                });
            }
        }

        // Handle Instruction Parsing
        let instructions = Self::order_instructions(&transaction_info);

        for program_instruction in instructions {
            match program_instruction {
                (program, instruction) if program == program_ids::gummy_roll_crud() => {
                    let maxlen = StreamMaxlen::Approx(5000);
                    let _message = match gummyroll_crud::get_instruction_type(&instruction.data) {
                        gummyroll_crud::InstructionName::CreateTree => {
                            warn!("yo yo yo yo");
                            let tree_id = keys.index(instruction.accounts[3] as usize);
                            let auth = keys.index(instruction.accounts[0] as usize);
                            let res: RedisResult<()> =
                                self.connection.as_mut().unwrap().xadd_maxlen(
                                    "GMC_OP",
                                    maxlen,
                                    "*",
                                    &[
                                        ("op", "create"),
                                        ("tree_id", &*tree_id.to_string()),
                                        ("authority", &*auth.to_string()),
                                    ],
                                );
                            if res.is_err() {
                                error!("{}", res.err().unwrap());
                            } else {
                                info!("Data Sent")
                            }
                        }
                        gummyroll_crud::InstructionName::Add => {
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let add: gummyroll_crud::instruction::Add =
                                gummyroll_crud::instruction::Add::deserialize(data_buf).unwrap();
                            let tree_id = keys.index(instruction.accounts[3] as usize);
                            let owner = keys.index(instruction.accounts[0] as usize);
                            let hex_message = hex::encode(&add.message);
                            let leaf = keccak::hashv(&[&owner.to_bytes(), add.message.as_slice()]);
                            let res: RedisResult<()> =
                                self.connection.as_mut().unwrap().xadd_maxlen(
                                    "GMC_OP",
                                    maxlen,
                                    "*",
                                    &[
                                        ("op", "add"),
                                        ("tree_id", &*tree_id.to_string()),
                                        ("leaf", &*leaf.to_string()),
                                        ("msg", &*hex_message),
                                        ("owner", &*owner.to_string()),
                                    ],
                                );
                            if res.is_err() {
                                error!("{}", res.err().unwrap());
                            } else {
                                info!("Data Sent")
                            }
                        }
                        gummyroll_crud::InstructionName::Transfer => {
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let add: gummyroll_crud::instruction::Transfer =
                                gummyroll_crud::instruction::Transfer::deserialize(data_buf)
                                    .unwrap();
                            let tree_id = keys.index(instruction.accounts[3] as usize);
                            let owner = keys.index(instruction.accounts[4] as usize);
                            let new_owner = keys.index(instruction.accounts[5] as usize);
                            let hex_message = hex::encode(&add.message);
                            let leaf =
                                keccak::hashv(&[&new_owner.to_bytes(), add.message.as_slice()]);
                            let res: RedisResult<()> =
                                self.connection.as_mut().unwrap().xadd_maxlen(
                                    "GMC_OP",
                                    maxlen,
                                    "*",
                                    &[
                                        ("op", "tran"),
                                        ("tree_id", &*tree_id.to_string()),
                                        ("leaf", &*leaf.to_string()),
                                        ("msg", &*hex_message),
                                        ("owner", &*owner.to_string()),
                                        ("new_owner", &*new_owner.to_string()),
                                    ],
                                );
                            if res.is_err() {
                                error!("{}", res.err().unwrap());
                            } else {
                                info!("Data Sent")
                            }
                        }
                        gummyroll_crud::InstructionName::Remove => {
                            let data = instruction.data[8..].to_owned();
                            let data_buf = &mut data.as_slice();
                            let remove: gummyroll_crud::instruction::Remove =
                                gummyroll_crud::instruction::Remove::deserialize(data_buf).unwrap();
                            let tree_id = keys.index(instruction.accounts[3] as usize);
                            let owner = keys.index(instruction.accounts[0] as usize);
                            let leaf = bs58::encode(&remove.leaf_hash).into_string();
                            let res: RedisResult<()> =
                                self.connection.as_mut().unwrap().xadd_maxlen(
                                    "GMC_OP",
                                    maxlen,
                                    "*",
                                    &[
                                        ("op", "rm"),
                                        ("tree_id", &*tree_id.to_string()),
                                        ("leaf", &*leaf.to_string()),
                                        ("msg", ""),
                                        ("owner", &*owner.to_string()),
                                    ],
                                );
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

    fn recv_account(&self) -> Result<()> {
        Ok(())
    }

    fn recv_transaction(&self) -> Result<()> {
        Ok(())
    }
}

impl Debug for RedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
