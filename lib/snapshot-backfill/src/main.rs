pub mod error;
use crate::error::SnapshotBackfillError;
use futures::{executor::block_on, future::join_all};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::pubkeys;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;

pubkeys!(
    BubblegumProgramID,
    "BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o"
);
pubkeys!(
    CandyMachineID,
    "cndyAnrLdpjq1Ssp1z8xxDsB8dxe7u4HL5Nxi2K5WXZ"
);

// Make sure this is ordering of sigs returned between these CM bounds
// 1. "4JrysfoZ6skfciFVQC3TXv7xPNShLtBF7nqvDrSfPt7JgzSsBrMyBVaTSX89b4kqxnb8kxM5jpm7Yi5v2ejFQQi7"
// 2. "64aLrNs7Q8ih4UovKCdp7cX68jCbuG2kYP5QsjenWZV9WKm3najCgpq3HUUqkcLshsPSWFWDfZxbJcYe1RtTJ9Hg"

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
struct Opt {
    #[structopt(
        short,
        default_value = "3yDUedPKUDT7JjMyrXzMRFPRmuVSpBjwwxFWqUeSj8HVs35w68hwFcPLhuzcVjmZsgJcbcHaqx4N6cbnPabQ1zqw"
    )]
    /// Exclusive bound
    before: String,
    #[structopt(
        short,
        default_value = "5yS1oaYhVs2ZBFx9wCryGnKMHvi1k6Mtioc6F1fw7K7rGrULCxFakq1uYJon7PDdw6xNeebPy4owo5WPR8kraCt"
    )]
    /// Exclusive bound
    until: String,
    #[structopt(
        short = "r",
        long = "rpc-url",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    url: String,
}

fn get_rpc_client(rpc_url: &str) -> Result<RpcClient, SnapshotBackfillError> {
    let client = RpcClient::new(rpc_url.to_string());
    Ok(client)
}

async fn parse_and_store_transactions(
    client: &RpcClient,
    opts: Opt,
) -> Result<(), SnapshotBackfillError> {
    let config = GetConfirmedSignaturesForAddress2Config {
        before: Some(Signature::new(
            bs58::decode(opts.before).into_vec()?.as_slice(),
        )),
        until: Some(Signature::new(
            bs58::decode(opts.until).into_vec()?.as_slice(),
        )),
        limit: None,
        commitment: Some(CommitmentConfig::confirmed()),
    };
    let sigs = client
        .get_signatures_for_address_with_config(&CandyMachineID(), config)
        .await?;
    println!("{:?}", sigs);
    let mut parse_results: Vec<_> = Vec::new();
    for sig_info in sigs.iter().rev() {
        parse_results.push(get_transaction(client, &sig_info.signature));
    }
    let results = join_all(parse_results).await;
    println!("{:?}", results);
    Ok(())
}

async fn get_transaction(client: &RpcClient, signature: &str) -> Result<(), SnapshotBackfillError> {
    println!("{:?}", signature);
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: None,
    };
    let tx = client
        .get_transaction_with_config(
            &Signature::new(bs58::decode(signature.to_string()).into_vec()?.as_slice()),
            config,
        )
        .await?;

    if let Some(meta) = tx.transaction.meta {
        if let Some(err) = meta.err {
            println!("{:?}", err);
            println!("Err found");
        } else {
            println!("No error found");
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Opt::from_args();
    println!("{:?}", &args);

    let client = get_rpc_client(&args.url).unwrap();
    parse_and_store_transactions(&client, args).await.unwrap();

    println!("Finished. Exiting...");
}
