use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlerkleError {
    #[error("Error connecting to the backend data store. Error message: ({msg})")]
    DataStoreConnectionError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    DataSchemaError { msg: String },

    #[error("Error preparing data store schema. Error message: ({msg})")]
    ConfigurationError { msg: String },

    #[error("Malformed Anchor Event")]
    EventError {},

    #[error("Unable to Send Event to Stream ({msg})")]
    EventStreamError { msg: String },
}
