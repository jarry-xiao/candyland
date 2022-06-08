pub mod error;
use crate::error::SnapshotBackfillError;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::pubkeys;
use solana_sdk::signature::Signature;
use structopt::StructOpt;

pubkeys!(
    BubblegumProgramID,
    "BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o"
);
pubkeys!(
    CandyMachineID,
    "cndyAnrLdpjq1Ssp1z8xxDsB8dxe7u4HL5Nxi2K5WXZ"
);

// Make sure this is the only sig returned between these CM bounds
// default_value = "64aLrNs7Q8ih4UovKCdp7cX68jCbuG2kYP5QsjenWZV9WKm3najCgpq3HUUqkcLshsPSWFWDfZxbJcYe1RtTJ9Hg"

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
        default_value = "4JrysfoZ6skfciFVQC3TXv7xPNShLtBF7nqvDrSfPt7JgzSsBrMyBVaTSX89b4kqxnb8kxM5jpm7Yi5v2ejFQQi7"
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

fn parse_and_store_transactions(client: RpcClient, opts: Opt) -> Result<(), SnapshotBackfillError> {
    let config = GetConfirmedSignaturesForAddress2Config {
        before: Some(Signature::new(
            bs58::decode(opts.before).into_vec()?.as_slice(),
        )),
        until: Some(Signature::new(
            bs58::decode(opts.until).into_vec()?.as_slice(),
        )),
        limit: Some(1),
        commitment: Some(CommitmentConfig::confirmed()),
    };
    let sigs = client.get_signatures_for_address_with_config(&CandyMachineID(), config)?;
    println!("{:?}", sigs);
    Ok(())
}

fn main() {
    let args = Opt::from_args();
    println!("{:?}", &args);

    let client = get_rpc_client(&args.url).unwrap();
    parse_and_store_transactions(client, args).unwrap();
}
