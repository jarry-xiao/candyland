use {
    crate::error::PlerkleError,
    log::*,
    messenger::Messenger,
    redis::{streams::StreamMaxlen, Commands, Connection, RedisResult},
    redis::{
        streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply},
        Value,
    },
    solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, Result},
    std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
    },
};

// Redis stream values.
const GROUP_NAME: &str = "plerkle";
const CONSUMER_NAME: &str = "ingester";
const DATA_KEY: &str = "data";

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
    streams: HashMap<&'static str, RedisMessengerStream>,
    stream_read_reply: StreamReadReply,
}

pub struct RedisMessengerStream {
    _buffer_size: StreamMaxlen,
    name: &'static str,
}

impl Messenger for RedisMessenger {
    fn new() -> Result<Self> {
        // Setup Redis client.
        let client = redis::Client::open("redis://redis/").unwrap();

        // Get connection.
        let connection = client.get_connection().map_err(|e| {
            error!("{}", e.to_string());
            GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                msg: e.to_string(),
            }))
        })?;

        Ok(Self {
            connection: Some(connection),
            streams: HashMap::<&'static str, RedisMessengerStream>::default(),
            stream_read_reply: StreamReadReply::default(),
        })
    }

    fn add_stream(&mut self, stream_key: &'static str, max_buffer_size: usize) {
        let _result = self.streams.insert(
            stream_key,
            RedisMessengerStream {
                name: stream_key,
                _buffer_size: StreamMaxlen::Approx(max_buffer_size),
            },
        );

        let created: core::result::Result<(), _> = self
            .connection
            .as_mut()
            .unwrap()
            .xgroup_create_mkstream(stream_key, GROUP_NAME, "$");
        if let Err(e) = created {
            println!("Group already exists: {:?}", e)
        }
    }

    fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<()> {
        if !self.streams.contains_key(stream_key) {
            error!("Cannot send. Stream key {stream_key} not configured");
            return Ok(());
        }

        // Put serialized data into Redis.
        let res: RedisResult<()> = self.connection.as_mut().unwrap().xadd_maxlen(
            stream_key,
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

    fn recv(&mut self) -> Result<()> {
        let opts = StreamReadOptions::default()
            .block(1000)
            .count(100000)
            .group(GROUP_NAME, CONSUMER_NAME);

        let keys: Vec<&str> = self.streams.keys().map(|s| *s).collect();
        let ids: Vec<&str> = vec![">"; keys.len()];

        self.stream_read_reply = self
            .connection
            .as_mut()
            .unwrap()
            .xread_options(&keys, &ids, &opts)
            .unwrap();

        Ok(())
    }

    fn get<'a>(&'a mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>> {
        let mut data_vec = Vec::<(i64, &[u8])>::new();

        let mut stream = if let Some(stream) = self.streams.get_mut(stream_key) {
            stream
        } else {
            error!("Cannot get data for stream key {stream_key}, it is not configured");
            return Ok(data_vec);
        };

        for StreamKey { key, ids } in self.stream_read_reply.keys.iter() {
            if key == stream_key {
                for StreamId { id, map } in ids {
                    let pid = id.replace("-", "").parse::<i64>().unwrap();

                    // Get data from map.
                    let data = if let Some(data) = map.get(DATA_KEY) {
                        data
                    } else {
                        println!("No Data was stored in Redis for ID {id}");
                        continue;
                    };
                    let bytes = match data {
                        Value::Data(bytes) => bytes,
                        _ => {
                            println!("Redis data for ID {id} in wrong format");
                            continue;
                        }
                    };

                    data_vec.push((pid, &bytes));
                }
            }
        }

        Ok(data_vec)
    }
}

impl Debug for RedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
