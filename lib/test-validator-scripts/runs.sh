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

# cat << EOL > accountsdb-plugin-config.json
# {
#     "libpath": "/plugin/plugin.so",
#     "accounts_selector" : {
#         "accounts" : [
#             "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
#             "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
#             "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD",
#             "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
#             "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
#             "BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o"
#         ]
#     },
#     "transaction_selector" : {
#         "mentions" : [
#             "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
#             "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
#             "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD",
#             "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",
#             "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",
#             "BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o"
#         ]
#     }
# }
# EOL
rm *so
ln -s ../../contracts/target/deploy/bubblegum.so bubblegum.so
ln -s ../../contracts/target/deploy/gummyroll_crud.so gummyroll_crud.so
ln -s ../../contracts/target/deploy/gummyroll.so gummyroll.so
ln -s ../../deps/metaplex-program-library/token-metadata/target/deploy/mpl_token_metadata.so mpl_token_metadata.so
# solana-program-library/associated-token-account/program && cargo build-bpf
ln -s ../../deps/solana-program-library/target/deploy/spl_associated_token_account.so spl_associated_token_account.so
# solana-program-library/token/program && cargo build-bpf
ln -s ../../deps/solana-program-library/target/deploy/spl_token.so spl_token.so
# solana-program-library/token/program-2022 && cargo build-bpf
ln -s ../../deps/solana-program-library/target/deploy/spl_token_2022.so spl_token_2022.so

export RUST_BACKTRACE=1
dataDir=$PWD/config/"$(basename "$0" .sh)"
ledgerDir=$PWD/config/ledger
mkdir -p "$dataDir" "$ledgerDir"
echo $ledgerDir
echo $dataDir
ls -la ./ 
args=(
  --config config.yaml
  --reset
  --rpc-port 8899
  --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s mpl_token_metadata.so
  --bpf-program BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o bubblegum.so
  --bpf-program Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS gummyroll_crud.so
  --bpf-program GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD gummyroll.so
  --bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA spl_token.so
  --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb spl_token_2022.so
  --bpf-program ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL spl_associated_token_account.so
)

#   --geyser-plugin-config accountsdb-plugin-config.json
# shellcheck disable=SC2086
# cat accountsdb-plugin-config.json
echo "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
solana-test-validator "${args[@]}" $SOLANA_RUN_SH_VALIDATOR_ARGS
