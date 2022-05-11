use solana_geyser_plugin_interface::geyser_plugin_interface::Result;

/// Some constants that can be used as stream key values.
pub const ACCOUNT_STREAM: &str = "ACC";
pub const SLOT_STREAM: &str = "SLT";
pub const TRANSACTION_STREAM: &str = "TXN";
pub const BLOCK_STREAM: &str = "BLK";

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn add_stream(&mut self, stream_key: &'static str, _max_buffer_size: usize);
    fn send(&mut self, stream_key: &'static str, bytes: &[u8]) -> Result<()>;
    fn recv(&mut self) -> Result<()>;
    fn get<'a>(&'a mut self, stream_key: &'static str) -> Result<Vec<(i64, &[u8])>>;
}
