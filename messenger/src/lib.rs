use {
    plerkle_serialization::PlerkleSerialized,
    solana_geyser_plugin_interface::geyser_plugin_interface::Result,
};

pub const DATA_KEY: &str = "data";

pub trait Messenger {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn send<'a, T: PlerkleSerialized<'a>>(&mut self, bytes: T) -> Result<()>;
}
