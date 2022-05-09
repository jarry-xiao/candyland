use solana_geyser_plugin_interface::geyser_plugin_interface::{
    ReplicaAccountInfo, ReplicaBlockInfo, ReplicaTransactionInfo, Result, SlotStatus,
};

pub const ACCOUNT_STREAM: &'static str = "ACC";
pub const SLOT_STREAM: &'static str = "SLT";
pub const TRANSACTION_STREAM: &'static str = "TXN";
pub const BLOCK_STREAM: &'static str = "BLK";
pub const DATA_KEY: &'static str = "data";

pub struct SerializedBlock<'a> {
    bytes: &'a [u8],
}

impl<'a> SerializedBlock<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    // TODO: Make these also take types like SerializedAccount, etc.
    // See SerializedBlock example.
    fn send_account(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_slot_status(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_transaction(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_block(&mut self, bytes: SerializedBlock) -> Result<()>;

    fn recv_account(&self) -> Result<()>;
    fn recv_slot_status(&self) -> Result<()>;
    fn recv_transaction(&self) -> Result<()>;
    fn recv_block(&self) -> Result<()>;
}

//fn send<T: MessengerKey>(&self, buffer: &[u8]) -> Result<()>;
//fn recv<'a, T: MessengerKey>(&self, buffer: &'a [u8]) -> Result<&'a [u8]>;
