use thiserror::Error;

#[derive(Error, Debug)]
pub enum MessengerError {
    #[error("Error creating connection: ({msg})")]
    ConnectionError { msg: String },

    #[error("Error sending data: ({msg})")]
    SendError { msg: String },

    #[error("Error receiving data: ({msg})")]
    ReceiveError { msg: String },
}
