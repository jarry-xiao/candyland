use std::collections::HashMap;
use redis::{Commands, Connection, ToRedisArgs};
use redis::streams::StreamMaxlen;
use solana_geyser_plugin_interface::geyser_plugin_interface::GeyserPluginError;
use crate::error::PlerkleError;
use solana_geyser_plugin_interface::geyser_plugin_interface::{
    Result
};

struct Messenger {
    connection: Connection,
    streams: HashMap<String, MessengerStream>,
}

struct MessengerStream {
    buffer_size: StreamMaxlen,
    name: String,
}

impl Messenger {
    pub fn add_stream(&mut self, name: String, max_buffer_size: usize) {
        self.streams.insert(name.clone(), MessengerStream {
            name,
            buffer_size: StreamMaxlen::Approx(max_buffer_size),
        });
    }

    pub fn get_stream(&self, name: String) -> Option<&MessengerStream> {
        self.streams.get(&*name)
    }


    pub fn add<K: ToRedisArgs, T: ToRedisArgs>(&mut self, stream: MessengerStream, id: String, items: &[(K, T)]) -> Result<()> {
        self.connection.xadd_maxlen(stream.name, stream.buffer_size, &*id, items)

            .map_err(|e| {
                GeyserPluginError::Custom(Box::new(PlerkleError::ConfigurationError {
                    msg: e.to_string(),
                }))
            })
    }
}