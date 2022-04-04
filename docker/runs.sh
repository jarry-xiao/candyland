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
    "libpath": "/so/plugin.so",
    "accounts_selector" : {
        "owners" : ["metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD"]
    },
    "transaction_selector" : {
        "mentions" : ["metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s", "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD"]
    }
}
EOL
export RUST_BACKTRACE=1
dataDir=$PWD/config/"$(basename "$0" .sh)"
ledgerDir=$PWD/config/ledger
mkdir -p "$dataDir" "$ledgerDir"
echo $ledgerDir
echo $dataDir
args=(
  --config config.yaml
  --log
  --reset
  --rpc-port 8899
  --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s /so/mpl_token_metadata.so
  --geyser-plugin-config accountsdb-plugin-config.json
)
# shellcheck disable=SC2086
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
