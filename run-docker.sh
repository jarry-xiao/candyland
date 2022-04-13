# Off Chain Setup
 # Speed up by using cargo remote
cross build --target x86_64-unknown-linux-gnu --package nft_api
cp target/x86_64-unknown-linux-gnu/debug/api target/debug/api
cp target/x86_64-unknown-linux-gnu/debug/ingest target/debug/ingest

# Validator setup
mkdir -p docker-vol
anchor build
cross build --target x86_64-unknown-linux-gnu --release --package plerkle
if [[ $(uname -m) =~ ^.*x86.*$ ]]; then
  cp target/release/libplerkle.so docker-vol/plugin.so
else
  cp target/x86_64-unknown-linux-gnu/release/libplerkle.so docker-vol/plugin.so
fi
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