use {
    figment::{Figment, providers::{Env}},
    serde::Deserialize,
};
use crate::error::DasApiError;

#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub metrics_port: u16,
    pub server_port: u16,
}

pub fn load_config() -> Result<Config, DasApiError> {
    Figment::new()
        .join(Env::prefixed("APP_"))
        .extract()
        .map_err(|config_error| {
            DasApiError::ConfigurationError(config_error.to_string())
        })
}