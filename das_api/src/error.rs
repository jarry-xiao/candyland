use {
    thiserror::Error
};

#[derive(Error, Debug)]
pub enum DasApiError {
    #[error("Config Missing or Error {0}")]
    ConfigurationError(String),
    #[error("Server Failed to Start")]
    ServerStartError(#[from] jsonrpsee_core::Error)
}