use solana_geyser_plugin_interface::geyser_plugin_interface::{
    ReplicaAccountInfo, ReplicaBlockInfo, ReplicaTransactionInfo, Result, SlotStatus,
};

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn send_account(&self, account: &ReplicaAccountInfo, slot: u64, is_startup: bool)
        -> Result<()>;
    fn send_slot_status(&self, slot: u64, parent: Option<u64>, status: SlotStatus) -> Result<()>;
    fn send_transaction(
        &mut self,
        transaction_info: &ReplicaTransactionInfo,
        slot: u64,
    ) -> Result<()>;
    fn send_block(&mut self, block_info: &ReplicaBlockInfo) -> Result<()>;
    fn recv_account(&self) -> Result<()>;
    fn recv_slot_status(&self) -> Result<()>;
    fn recv_transaction(&self) -> Result<()>;
    fn recv_block(&self) -> Result<()>;
}
