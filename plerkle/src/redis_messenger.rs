use {
    crate::{
        account_info_generated::account_info::{AccountInfo, AccountInfoArgs, root_as_account_info},
        block_info_generated,
        error::PlerkleError,
        programs::gummy_roll::handle_change_log_event,
        slot_status_info_generated::slot_status_info::{self, SlotStatusInfo, SlotStatusInfoArgs},
        transaction_info_generated::transaction_info::{
            self, TransactionInfo, TransactionInfoArgs,
        },
    },
    anchor_client::anchor_lang::AnchorDeserialize,
    flatbuffers::FlatBufferBuilder,
    log::*,
    messenger::Messenger,
    redis::{streams::StreamMaxlen, Commands, Connection, RedisResult, ToRedisArgs},
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, ReplicaAccountInfo, ReplicaBlockInfo, ReplicaTransactionInfo, Result,
        SlotStatus,
    },
    solana_runtime::bank::RewardType,
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

    fn send_account(
        &self,
        account: &ReplicaAccountInfo,
        slot: u64,
        is_startup: bool,
    ) -> Result<()> {
        let mut builder = FlatBufferBuilder::new();

        let pubkey = builder.create_vector(account.pubkey);
        let owner = builder.create_vector(account.owner);
        let data = builder.create_vector(account.data);

        let account_info = AccountInfo::create(
            &mut builder,
            &AccountInfoArgs {
                pubkey: Some(pubkey),
                lamports: account.lamports,
                owner: Some(owner),
                executable: account.executable,
                rent_epoch: account.rent_epoch,
                data: Some(data),
                write_version: account.write_version,
                slot,
                is_startup,
            },
        );

        builder.finish(account_info, None);
        let _serialized_data = builder.finished_data();

        Ok(())
    }

    fn send_slot_status(&self, slot: u64, parent: Option<u64>, status: SlotStatus) -> Result<()> {
        let mut builder = FlatBufferBuilder::new();

        let status = match status {
            SlotStatus::Confirmed => slot_status_info::Status::Confirmed,
            SlotStatus::Processed => slot_status_info::Status::Processed,
            SlotStatus::Rooted => slot_status_info::Status::Rooted,
        };

        let slot_status = SlotStatusInfo::create(
            &mut builder,
            &SlotStatusInfoArgs {
                slot,
                parent,
                status,
            },
        );

        builder.finish(slot_status, None);
        let _serialized_data = builder.finished_data();

        Ok(())
    }

    fn send_transaction(
        &mut self,
        transaction_info: &ReplicaTransactionInfo,
        slot: u64,
    ) -> Result<()> {
        let mut builder = FlatBufferBuilder::new();

        // Flatten and serialize account keys.
        let account_keys = transaction_info.transaction.message().account_keys();
        let account_keys_len = account_keys.len();

        let account_keys = if account_keys_len > 0 {
            let mut account_keys_fb_vec = Vec::with_capacity(account_keys_len);
            for key in account_keys.iter() {
                account_keys_fb_vec.push(transaction_info::Pubkey::new(&key.to_bytes()));
            }
            Some(builder.create_vector(&account_keys_fb_vec))
        } else {
            None
        };

        // Serialize log messages.
        let log_messages = if let Some(log_messages) = transaction_info
            .transaction_status_meta
            .log_messages
            .as_ref()
        {
            let mut log_messages_fb_vec = Vec::with_capacity(log_messages.len());
            for message in log_messages {
                log_messages_fb_vec.push(builder.create_string(&message));
            }
            Some(builder.create_vector(&log_messages_fb_vec))
        } else {
            None
        };

        // Serialize inner instructions.
        let inner_instructions = if let Some(inner_instructions_vec) = transaction_info
            .transaction_status_meta
            .inner_instructions
            .as_ref()
        {
            let mut overall_fb_vec = Vec::with_capacity(inner_instructions_vec.len());
            for inner_instructions in inner_instructions_vec.iter() {
                let index = inner_instructions.index;
                let mut instructions_fb_vec =
                    Vec::with_capacity(inner_instructions.instructions.len());
                for compiled_instruction in inner_instructions.instructions.iter() {
                    let program_id_index = compiled_instruction.program_id_index;
                    let accounts = Some(builder.create_vector(&compiled_instruction.accounts));
                    let data = Some(builder.create_vector(&compiled_instruction.data));
                    instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                        &mut builder,
                        &transaction_info::CompiledInstructionArgs {
                            program_id_index,
                            accounts,
                            data,
                        },
                    ));
                }

                let instructions = Some(builder.create_vector(&instructions_fb_vec));
                overall_fb_vec.push(transaction_info::InnerInstructions::create(
                    &mut builder,
                    &transaction_info::InnerInstructionsArgs {
                        index,
                        instructions,
                    },
                ))
            }

            Some(builder.create_vector(&overall_fb_vec))
        } else {
            None
        };

        // Serialize outer instructions.
        let outer_instructions = transaction_info.transaction.message().instructions();
        let outer_instructions = if outer_instructions.len() > 0 {
            let mut instructions_fb_vec = Vec::with_capacity(outer_instructions.len());
            for compiled_instruction in outer_instructions.iter() {
                let program_id_index = compiled_instruction.program_id_index;
                let accounts = Some(builder.create_vector(&compiled_instruction.accounts));
                let data = Some(builder.create_vector(&compiled_instruction.data));
                instructions_fb_vec.push(transaction_info::CompiledInstruction::create(
                    &mut builder,
                    &transaction_info::CompiledInstructionArgs {
                        program_id_index,
                        accounts,
                        data,
                    },
                ));
            }
            Some(builder.create_vector(&instructions_fb_vec))
        } else {
            None
        };

        // Serialize everything into Transaction Info.
        let transaction_info = TransactionInfo::create(
            &mut builder,
            &TransactionInfoArgs {
                is_vote: transaction_info.is_vote,
                account_keys,
                log_messages,
                inner_instructions,
                outer_instructions,
                slot,
            },
        );

        builder.finish(transaction_info, None);
        let serialized_data = builder.finished_data();

        // Put serialized data into Redis.
        let res: RedisResult<()> = self.connection.as_mut().unwrap().xadd_maxlen(
            "TX",
            StreamMaxlen::Approx(55000),
            "*",
            &[("data", serialized_data)],
        );
        if res.is_err() {
            error!("{}", res.err().unwrap());
        } else {
            info!("Data Sent");
        }

        Ok(())
    }

    fn send_block(&mut self, block_info: &ReplicaBlockInfo) -> Result<()> {
        let mut builder = FlatBufferBuilder::new();

        let blockhash = Some(builder.create_string(&block_info.blockhash));
        let rewards = if block_info.rewards.len() > 0 {
            let mut rewards_fb_vec = Vec::with_capacity(block_info.rewards.len());
            for reward in block_info.rewards.iter() {
                let pubkey = Some(builder.create_vector(reward.pubkey.as_bytes()));
                let lamports = reward.lamports;
                let post_balance = reward.post_balance;

                let reward_type = if let Some(reward) = reward.reward_type {
                    match reward {
                        RewardType::Fee => Some(block_info_generated::block_info::RewardType::Fee),
                        RewardType::Rent => {
                            Some(block_info_generated::block_info::RewardType::Rent)
                        }
                        RewardType::Staking => {
                            Some(block_info_generated::block_info::RewardType::Staking)
                        }
                        RewardType::Voting => {
                            Some(block_info_generated::block_info::RewardType::Voting)
                        }
                    }
                } else {
                    None
                };

                let commission = reward.commission;

                rewards_fb_vec.push(block_info_generated::block_info::Reward::create(
                    &mut builder,
                    &block_info_generated::block_info::RewardArgs {
                        pubkey,
                        lamports,
                        post_balance,
                        reward_type,
                        commission,
                    },
                ));
            }
            Some(builder.create_vector(&rewards_fb_vec))
        } else {
            None
        };

        let block_info = block_info_generated::block_info::BlockInfo::create(
            &mut builder,
            &block_info_generated::block_info::BlockInfoArgs {
                slot: block_info.slot,
                blockhash,
                rewards,
                block_time: block_info.block_time,
                block_height: block_info.block_height,
            },
        );

        builder.finish(block_info, None);

        let _serialized_data = builder.finished_data();

        Ok(())
    }

    fn recv_account(&self) -> Result<()> {
        Ok(())
    }

    fn recv_slot_status(&self) -> Result<()> {
        Ok(())
    }

    fn recv_transaction(&self) -> Result<()> {
        Ok(())
    }

    fn recv_block(&self) -> Result<()> {
        Ok(())
    }
}

impl Debug for RedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
