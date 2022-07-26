mod api;
mod api_impl;
mod config;
mod error;
mod validation;

use crate::api::RpcApiBuilder;
use crate::api_impl::DasApi;
use {
    crate::config::load_config,
    crate::error::DasApiError,
    jsonrpsee::http_server::{HttpServerBuilder, RpcModule},
    std::net::SocketAddr,
    tokio,
};

#[tokio::main]
async fn main() -> Result<(), DasApiError> {
    let config = load_config()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server_port));
    let server = HttpServerBuilder::default().build(addr).await?;
    let api = DasApi::from_config(config).await?;
    let rpc = RpcApiBuilder::build(Box::new(api))?;
    println!("Server Started");
    server.start(rpc)?.await;
    println!("Server ended");
    Ok(())
}
