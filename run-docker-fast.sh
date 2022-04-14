# Off Chain Setup
 # Speed up by using cargo remote
cargo remote -c debug/api -- build --package nft_api
cargo remote -c debug/ingest -- build --package nft_api


# Validator setup
anchor build
cargo remote -c release/libplerkle.so -- build --release --package plerkle
mkdir -p docker-vol
cp target/release/libplerkle.so docker-vol/plugin.so # if on mac you ned the lin unknown target in front
cp target/deploy/merkle_wallet.so docker-vol/merkle.so
cp target/deploy/gummyroll.so docker-vol/gummyroll.so
cp target/deploy/gummyroll_crud.so docker-vol/gummyroll_crud.so

pushd deps/metaplex-program-library/token-metadata/program
  cargo build-bpf --bpf-out-dir ../../target/deploy/
popd
pushd deps/solana-program-library/associated-token-account
  cargo build-bpf
popd
pushd deps/solana-program-library/token/program-2022
  cargo build-bpf
popd

cp deps/metaplex-program-library/target/deploy/mpl_token_metadata.so docker-vol/mpl_token_metadata.so
cp deps/solana-program-library/target/deploy/spl_token_2022.so docker-vol/spl_token_2022.so
cp deps/solana-program-library/target/deploy/spl_associated_token_account.so docker-vol/spl_associated_token_account.so

# speed this up somehow
docker-compose up --build --force-recreate