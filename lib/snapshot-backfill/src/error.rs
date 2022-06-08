use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapshotBackfillError {
    #[error("Config provided fails to create valid RPC client")]
    ConfigInvalid,
    #[error("Could not decode string into base58 bytes")]
    Base58DecodeError,
    #[error("Error using RPC Client")]
    RpcClientError,
}

impl From<bs58::decode::Error> for SnapshotBackfillError {
    fn from(err: bs58::decode::Error) -> Self {
        println!("{:?}", err);
        SnapshotBackfillError::Base58DecodeError
    }
}

impl From<solana_client::client_error::ClientError> for SnapshotBackfillError {
    fn from(err: solana_client::client_error::ClientError) -> Self {
        println!("{:?}", err);
        SnapshotBackfillError::RpcClientError
    }
}
