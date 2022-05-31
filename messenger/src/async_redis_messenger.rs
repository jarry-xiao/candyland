#![cfg(feature = "redis")]

use {
    crate::{error::MessengerError, Messenger},
    async_trait::async_trait,
    log::*,
    redis::{
        aio::AsyncStream,
        streams::{StreamId, StreamKey, StreamMaxlen, StreamReadOptions, StreamReadReply},
        AsyncCommands, RedisResult, Value,
    },
    solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, Result},
    std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
        pin::Pin,
    },
};

// Redis stream values.
pub const GROUP_NAME: &str = "plerkle";
pub const CONSUMER_NAME: &str = "ingester";
pub const DATA_KEY: &str = "data";

#[derive(Default)]
pub struct AsyncRedisMessenger {
    connection: Option<redis::aio::Connection<Pin<Box<dyn AsyncStream + Send + Sync>>>>,
    streams: HashMap<&'static str, RedisMessengerStream>,
    stream_read_reply: StreamReadReply,
}

pub struct RedisMessengerStream {
    buffer_size: Option<StreamMaxlen>,
}

#[async_trait]
impl Messenger for AsyncRedisMessenger {
    //pub async fn new(stream_key: &'static str) -> Result<Self> {
    async fn new() -> Result<Self> {
        // Setup Redis client.
        let client = redis::Client::open("redis://redis/").unwrap();

        // Get connection.
        let connection = client.get_tokio_connection().await.map_err(|e| {
            error!("{}", e.to_string());
            GeyserPluginError::Custom(Box::new(MessengerError::ConfigurationError {
                msg: e.to_string(),
            }))
        })?;

        Ok(Self {
            connection: Some(connection),
            streams: HashMap::<&'static str, RedisMessengerStream>::default(),
            stream_read_reply: StreamReadReply::default(),
        })
    }

    async fn add_stream(&mut self, stream_key: &'static str) {
        // Add to streams hashmap.
        let _result = self
            .streams
            .insert(stream_key, RedisMessengerStream { buffer_size: None });

        // Add stream to Redis.
        let result: RedisResult<()> = self
            .connection
            .as_mut()
            .unwrap()
            .xgroup_create_mkstream(stream_key, GROUP_NAME, "$")
            .await;

        if let Err(e) = result {
            println!("Group already exists: {:?}", e)
        }
    }

    async fn set_buffer_size(&mut self, stream_key: &'static str, max_buffer_size: usize) {
        // Set max length for the stream.
        if let Some(stream) = self.streams.get_mut(stream_key) {
            stream.buffer_size = Some(StreamMaxlen::Approx(max_buffer_size));
        } else {
            error!("Stream key {stream_key} not configured");
        }
    }

    //impl Future<Output = Result<()>> + 'a {
    async fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<()> {
        // Check if stream is configured.
        let stream = if let Some(stream) = self.streams.get(stream_key) {
            stream
        } else {
            error!("Cannot send data for stream key {stream_key}, it is not configured");
            return Ok(());
        };

        // Get max length for the stream.
        let maxlen = if let Some(maxlen) = stream.buffer_size {
            maxlen
        } else {
            error!("Cannot send data for stream key {stream_key}, buffer size not set.");
            return Ok(());
        };

        // Put serialized data into Redis.
        let result: RedisResult<()> = self
            .connection
            .as_mut()
            .unwrap()
            .xadd_maxlen(stream_key, maxlen, "*", &[(DATA_KEY, &bytes)])
            .await;

        // Log but do not return errors.
        if let Err(e) = result {
            error!("Redis send error: {e}");
        } else {
            info!("Data Sent");
        }

        Ok(())
    }

    async fn recv(&mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>> {
        let opts = StreamReadOptions::default()
            .block(0) // Block forever.
            .count(1) // Get one item.
            .group(GROUP_NAME, CONSUMER_NAME);

        // Read on stream key and save the reply. Log but do not return errors.
        match self
            .connection
            .as_mut()
            .unwrap()
            .xread_options(&[stream_key], &[">"], &opts)
            .await
        {
            Ok(reply) => self.stream_read_reply = reply,
            Err(e) => error!("Redis receive error: {e}"),
        }

        // Data vec that will be returned with parsed data from stream read reply.
        let mut data_vec = Vec::<(i64, &[u8])>::new();

        // Parse data in stream read reply and store in Vec to return to caller.
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

impl Debug for AsyncRedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
