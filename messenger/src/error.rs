use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessengerError {
    #[error("Missing or invalid configuration: ({msg})")]
    ConfigurationError { msg: String },

    #[error("Error creating connection: ({msg})")]
    ConnectionError { msg: String },

    #[error("Error sending data: ({msg})")]
    SendError { msg: String },

    #[error("Error receiving data: ({msg})")]
    ReceiveError { msg: String },
}
