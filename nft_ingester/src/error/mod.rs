
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IngesterError {
    #[error("ChangeLog Event Malformed")]
    ChangeLogEventMalformed,
    #[error("Error downloading batch files")]
    BatchInitNetworkingError,
    #[error("Error writing batch files")]
    BatchInitIOError,
}

impl From<reqwest::Error> for IngesterError {
    fn from(_err: reqwest::Error) -> Self {
        IngesterError::BatchInitNetworkingError
    }
}

impl From<std::io::Error> for IngesterError {
    fn from(_err: std::io::Error) -> Self {
        IngesterError::BatchInitIOError
    }
}
