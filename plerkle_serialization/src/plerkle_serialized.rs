/// Some constnats that can be used as PlerkleSerialized key values.
pub const ACCOUNT_STREAM: &str = "ACC";
pub const SLOT_STREAM: &str = "SLT";
pub const TRANSACTION_STREAM: &str = "TXN";
pub const BLOCK_STREAM: &str = "BLK";

/// This trait indicates data was serialized and supports the
/// included methods to retrieve the serialized bytes and an
/// arbitrary string storage key that can be used by other storage
/// methods.
pub trait PlerkleSerialized<'a> {
    fn new(bytes: &'a [u8]) -> Self;
    fn bytes(&self) -> &'a [u8];
    fn key(&self) -> &'static str;
}
