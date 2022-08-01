#![cfg(feature = "redis")]
use {
    crate::{error::MessengerError, Messenger, MessengerConfig},
    async_trait::async_trait,
    log::*,
    redis::{
        aio::AsyncStream,
        streams::{StreamId, StreamKey, StreamMaxlen, StreamReadOptions, StreamReadReply},
        AsyncCommands, RedisResult, Value,
    },
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
pub struct RedisMessenger {
    connection: Option<redis::aio::Connection<Pin<Box<dyn AsyncStream + Send + Sync>>>>,
    streams: HashMap<&'static str, RedisMessengerStream>,
    stream_read_reply: StreamReadReply,
}

pub struct RedisMessengerStream {
    buffer_size: Option<StreamMaxlen>,
}

const REDIS_CON_STR: &str = "redis_connection_str";

#[async_trait]
impl Messenger for RedisMessenger {
    //pub async fn new(stream_key: &'static str) -> Result<Self> {
    async fn new(config: MessengerConfig) -> Result<Self, MessengerError> {
        let uri = config.get(&*REDIS_CON_STR)
            .and_then(|u| u.clone().into_string())
            .ok_or(MessengerError::ConfigurationError { msg: format!("Connection String Missing: {}", REDIS_CON_STR) })?;
        // Setup Redis client.
        let client = redis::Client::open(uri).unwrap();

        // Get connection.
        let connection = client.get_tokio_connection().await.map_err(|e| {
            error!("{}", e.to_string());
            MessengerError::ConnectionError {
                msg: e.to_string(),
            }
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

    async fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<(), MessengerError> {
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

        if let Err(e) = result {
            error!("Redis send error: {e}");
            return Err(
                MessengerError::SendError { msg: e.to_string() }
            );
        } else {
            info!("Data Sent to {}", stream_key);
        }

        Ok(())
    }

    async fn recv(&mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>, MessengerError> {
        let opts = StreamReadOptions::default()
            .block(0) // Block forever.
            .count(1) // Get one item.
            .group(GROUP_NAME, CONSUMER_NAME);

        // Read on stream key and save the reply. Log but do not return errors.
        self.stream_read_reply = match self
            .connection
            .as_mut()
            .unwrap()
            .xread_options(&[stream_key], &[">"], &opts)
            .await
        {
            Ok(reply) => reply,
            Err(e) => {
                error!("Redis receive error: {e}");
                return Err(
                    MessengerError::ReceiveError { msg: e.to_string() }
                );
            }
        };

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

impl Debug for RedisMessenger {
    fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}
