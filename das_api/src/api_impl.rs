use crate::config::Config;
use crate::DasApiError;

pub struct DasApi {
    db_connection: DatabaseConnection
}

impl DasApi {
    pub fn from_config(config: Config) -> Result<Self, DasApiError> {

    }
}