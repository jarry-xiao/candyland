use {
    crate::error::PlerkleError,
    log::*,
    messenger::{Messenger, DATA_KEY},
    plerkle_serialization::PlerkleSerialized,
    redis::{streams::StreamMaxlen, Commands, Connection, RedisResult, ToRedisArgs},
    solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, Result},
    std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
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

    fn send<'a, T: PlerkleSerialized<'a>>(&mut self, bytes: T) -> Result<()> {
        // Put serialized data into Redis.
        let res: RedisResult<()> = self.connection.as_mut().unwrap().xadd_maxlen(
            bytes.key(),
            StreamMaxlen::Approx(55000),
            "*",
            &[(DATA_KEY, bytes.bytes())],
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
