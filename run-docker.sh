#!/bin/bash
# Exit on failure
set -e
export RUSTFLAGS='--cfg procmacro2_semver_exempt'
# Build on-chain programs first
anchor build
cp target/deploy/gummyroll.so docker-vol/gummyroll.so
cp target/deploy/gummyroll_crud.so docker-vol/gummyroll_crud.so
cp target/deploy/bubblegum.so docker-vol/bubblegum.so

# Off Chain Setup
# Speed up by using cargo remote
cargo build --target x86_64-unknown-linux-gnu --package nft-api
cp target/x86_64-unknown-linux-gnu/debug/api target/debug/api
cargo build --target x86_64-unknown-linux-gnu --package nft-ingester
cp target/x86_64-unknown-linux-gnu/debug/ingest target/debug/ingest

# Validator setup
mkdir -p docker-vol
cargo build --target x86_64-unknown-linux-gnu --release --package plerkle
cp target/x86_64-unknown-linux-gnu/release/libplerkle.so docker-vol/plugin.so # if on mac you ned the lin unknown target in front

pushd deps/metaplex-program-library/token-metadata/program
  cargo build-bpf --bpf-out-dir ../../target/deploy/
popd
pushd deps/solana-program-library/associated-token-account
  cargo build-bpf
popd
pushd deps/solana-program-library/token/program-2022
  cargo build-bpf
popd
pushd deps/solana-program-library/token/program
  cargo build-bpf
popd

cp deps/metaplex-program-library/target/deploy/mpl_token_metadata.so docker-vol/mpl_token_metadata.so
cp deps/solana-program-library/target/deploy/spl_token_2022.so docker-vol/spl_token_2022.so
cp deps/solana-program-library/target/deploy/spl_token.so docker-vol/spl_token.so
cp deps/solana-program-library/target/deploy/spl_associated_token_account.so docker-vol/spl_associated_token_account.so

echo "----------------------------------------------------"
echo "If you got to this point, everything built & is fine"
echo "... now just need to run docker-compose up --build --force-recreate"

# speed this up somehow
# this won't succeed unless `sudo rm -rf db-data` has been executed
docker-compose up --build --force-recreate
