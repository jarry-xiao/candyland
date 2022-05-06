use hyper::StatusCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("ChangeLog Event Malformed")]
    ChangeLogEventMalformed,
    #[error("Parameter Invalid")]
    ParameterInvalid,
    #[error("Error downloading batch files")]
    BatchInitNetworkingError,
    #[error("Error writing batch files")]
    BatchInitIOError,
    #[error("Request Error {status:?}, reason {msg:?} ")]
    ResponseError { status: StatusCode, msg: String },
}

impl From<reqwest::Error> for ApiError {
    fn from(_err: reqwest::Error) -> Self {
        ApiError::BatchInitNetworkingError
    }
}

impl From<std::io::Error> for ApiError {
    fn from(_err: std::io::Error) -> Self {
        ApiError::BatchInitIOError
    }
}
