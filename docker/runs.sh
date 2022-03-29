#!/usr/bin/env bash
#
# Run a minimal Solana cluster.  Ctrl-C to exit.
#
# Before running this script ensure standard Solana programs are available
# in the PATH, or that `cargo build` ran successfully
# change
#
set -e
cat << EOL > config.yaml
json_rpc_url: http://localhost:8899
websocket_url: ws://localhost:8899
commitment: finalized
EOL

cat << EOL > accountsdb-plugin-config.json
{
    "libpath": "./plugin.so",
    "accounts_selector" : {
        "owners" : ["metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "4L5JPAZyZ9eijKREpLAKGNNjCYVX5e1H53DvvweiMNnG"]
    },
    "transaction_selector" : {
        "mentions" : ["metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "4L5JPAZyZ9eijKREpLAKGNNjCYVX5e1H53DvvweiMNnG"]
    }
}
EOL
export RUST_LOG=${RUST_LOG=debug} # if RUST_LOG is unset, default to info
export RUST_BACKTRACE=1
dataDir=$PWD/config/"$(basename "$0" .sh)"
ledgerDir=$PWD/config/ledger
mkdir -p "$dataDir" "$ledgerDir"
args=(
  --config config.yaml
  --log
  --reset
  --rpc-port 8899
  --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s mpl_token_metadata.so
  --bpf-program "4L5JPAZyZ9eijKREpLAKGNNjCYVX5e1H53DvvweiMNnG" gummyroll.so
  --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb spl_token_2022.so
  --bpf-program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL spl_associated_token_account.so
  --accountsdb-plugin-config accountsdb-plugin-config.json
)
# shellcheck disable=SC2086
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS &
validator=$!
#solana-keygen new --outfile validator.key
#PUB=`solana-keygen pubkey validator.key`
#solana -C config.yaml airdrop 1 $PUB
wait "$validator"