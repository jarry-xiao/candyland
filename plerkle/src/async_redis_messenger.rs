use {
    crate::{
        constants::{CONSUMER_NAME, DATA_KEY, GROUP_NAME},
        error::PlerkleError,
    },
    log::*,
    redis::{
        aio::AsyncStream,
        streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply},
        AsyncCommands, Value,
    },
    solana_geyser_plugin_interface::geyser_plugin_interface::{GeyserPluginError, Result},
    std::{
        fmt::{Debug, Formatter},
        pin::Pin,
    },
};
pub struct AsyncRedisMessenger {
    connection: Option<redis::aio::Connection<Pin<Box<dyn AsyncStream + Send + Sync>>>>,
    stream_key: &'static str,
    stream_read_reply: StreamReadReply,
}

impl AsyncRedisMessenger {
    pub async fn new(stream_key: &'static str) -> Result<Self> {
        // Setup Redis client.
        let client = redis::Client::open("redis://redis/").unwrap();

        // Get connection.
        let connection = client.get_tokio_connection().await.map_err(|e| {
            error!("{}", e.to_string());
            GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                msg: e.to_string(),
            }))
        })?;

        Ok(Self {
            connection: Some(connection),
            stream_key,
            stream_read_reply: StreamReadReply::default(),
        })
    }

    pub async fn recv(&mut self) -> Result<Vec<(i64, &[u8])>> {
        let opts = StreamReadOptions::default()
            .block(0) // Block forever.
            .count(1) // Get one item.
            .group(GROUP_NAME, CONSUMER_NAME);

        // Read on stream key and save the reply. Log but do not return errors.
        match self
            .connection
            .as_mut()
            .unwrap()
            .xread_options(&[self.stream_key], &[">"], &opts)
            .await
        {
            Ok(reply) => self.stream_read_reply = reply,
            Err(e) => error!("Redis receive error: {e}"),
        }

        // Data vec that will be returned with parsed data from stream read reply.
        let mut data_vec = Vec::<(i64, &[u8])>::new();

        // Parse data in stream read reply and store in Vec to return to caller.
        for StreamKey { key, ids } in self.stream_read_reply.keys.iter() {
            if key == self.stream_key {
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
