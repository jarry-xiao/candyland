use {
    crate::error::PlerkleError,
    log::*,
    messenger::{
        Messenger, SerializedBlock, ACCOUNT_STREAM, BLOCK_STREAM, DATA_KEY, SLOT_STREAM,
        TRANSACTION_STREAM,
    },
    redis::{streams::StreamMaxlen, Commands, Connection, RedisResult, ToRedisArgs},
    solana_geyser_plugin_interface::geyser_plugin_interface::{
        GeyserPluginError, ReplicaTransactionInfo, Result,
    },
    solana_sdk::{instruction::CompiledInstruction, pubkey::Pubkey},
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

    fn send_account(&mut self, bytes: &[u8]) -> Result<()> {
        self.send_data(ACCOUNT_STREAM, bytes)
    }

    fn send_slot_status(&mut self, bytes: &[u8]) -> Result<()> {
        self.send_data(SLOT_STREAM, bytes)
    }

    fn send_transaction(&mut self, bytes: &[u8]) -> Result<()> {
        self.send_data(TRANSACTION_STREAM, bytes)
    }

    fn send_block(&mut self, bytes: SerializedBlock) -> Result<()> {
        self.send_data(BLOCK_STREAM, bytes.bytes())
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

impl RedisMessenger {
    fn send_data(&mut self, stream_name: &'static str, bytes: &[u8]) -> Result<()> {
        // Put serialized data into Redis.
        let res: RedisResult<()> = self.connection.as_mut().unwrap().xadd_maxlen(
            stream_name,
            StreamMaxlen::Approx(55000),
            "*",
            &[(DATA_KEY, bytes)],
        );

        // Log but do not return errors.
        if res.is_err() {
            error!("{}", res.err().unwrap());
        } else {
            info!("Data Sent");
        }

        Ok(())
    }
}

impl Debug for RedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
