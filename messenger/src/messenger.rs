use figment::value::Dict;
use serde::Deserialize;
use {async_trait::async_trait};
use crate::error::MessengerError;

/// Some constants that can be used as stream key values.
pub const ACCOUNT_STREAM: &str = "ACC";
pub const SLOT_STREAM: &str = "SLT";
pub const TRANSACTION_STREAM: &str = "TXN";
pub const BLOCK_STREAM: &str = "BLK";

#[async_trait]
pub trait Messenger: Sync + Send {
    async fn new(config: MessengerConfig) -> Result<Self, MessengerError>
    where
        Self: Sized;

    async fn add_stream(&mut self, stream_key: &'static str);
    async fn set_buffer_size(&mut self, stream_key: &'static str, max_buffer_size: usize);
    async fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<(), MessengerError>;
    async fn recv(&mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>, MessengerError>;
}

pub type MessengerConfig = Dict;
