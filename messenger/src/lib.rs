use solana_geyser_plugin_interface::geyser_plugin_interface::{
    ReplicaAccountInfo, ReplicaBlockInfo, ReplicaTransactionInfo, Result, SlotStatus,
};

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn send_account(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_slot_status(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_transaction(&mut self, bytes: &[u8]) -> Result<()>;
    fn send_block(&mut self, bytes: &[u8]) -> Result<()>;

    fn recv_account(&self) -> Result<()>;
    fn recv_slot_status(&self) -> Result<()>;
    fn recv_transaction(&self) -> Result<()>;
    fn recv_block(&self) -> Result<()>;
}

//fn send<T: MessengerKey>(&self, buffer: &[u8]) -> Result<()>;
//fn recv<'a, T: MessengerKey>(&self, buffer: &'a [u8]) -> Result<&'a [u8]>;
