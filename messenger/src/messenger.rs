use solana_geyser_plugin_interface::geyser_plugin_interface::Result;

/// Some constants that can be used as stream key values.
pub const ACCOUNT_STREAM: &str = "ACC";
pub const SLOT_STREAM: &str = "SLT";
pub const TRANSACTION_STREAM: &str = "TXN";
pub const BLOCK_STREAM: &str = "BLK";

use async_trait::async_trait;

#[async_trait]
//TODO do I need sync and send.
pub trait Messenger: Sync + Send {
    async fn new() -> Result<Self>
    where
        Self: Sized;

    async fn add_stream(&mut self, stream_key: &'static str);
    async fn set_buffer_size(&mut self, stream_key: &'static str, max_buffer_size: usize);
    async fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<()>;
    async fn recv(&mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>>;
    //async fn get<'a>(&'a mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>>;
}
