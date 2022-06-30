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
    "libpath": "/plugin/plugin.so",
    "accounts_selector" : {
        "accounts" : [
            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
            "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
            "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"
        ]
    },
    "transaction_selector" : {
        "mentions" : [
            "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
            "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
            "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD",
            "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
            "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
            "BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY"
        ]
    }
}
EOL
export RUST_BACKTRACE=1
dataDir=$PWD/config/"$(basename "$0" .sh)"
ledgerDir=$PWD/config/ledger
mkdir -p "$dataDir" "$ledgerDir"
echo $ledgerDir
echo $dataDir
ls -la /so/
args=(
  --config config.yaml
  --log
  --reset
  --rpc-port 8899
  --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s /so/mpl_token_metadata.so
  --bpf-program BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY /so/bubblegum.so
  --bpf-program GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD /so/gummyroll.so
  --bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA /so/spl_token.so
  --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb /so/spl_token_2022.so
  --bpf-program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL /so/spl_associated_token_account.so
  --bpf-program WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh /so/candy_wrapper.so
  --geyser-plugin-config accountsdb-plugin-config.json
)
# shellcheck disable=SC2086
cat accountsdb-plugin-config.json
echo "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
