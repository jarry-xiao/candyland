anchor build
pushd plerkle
  cross build --target x86_64-unknown-linux-gnu
popd
cp deps/accountsdb-postgres/target/x86_64-unknown-linux-gnu/debug/libsolana_accountsdb_plugin_postgres.so docker-vol/psl.so
cp plerkle/target/x86_64-unknown-linux-gnu/debug/libplerkle.so docker-vol/plugin.so
cp target/deploy/merkle_wallet.so docker-vol/merkle.so
cp target/deploy/gummyroll.so docker-vol/gummyroll.so
cp target/deploy/gummyroll_crud.so docker-vol/gummyroll_crud.so
cp deps/metaplex-program-library/target/deploy/mpl_token_metadata.so docker-vol/mpl_token_metadata.so
cp deps/solana-program-library/target/deploy/spl_token_2022.so docker-vol/spl_token_2022.so
cp deps/solana-program-library/target/deploy/spl_associated_token_account.so docker-vol/spl_associated_token_account.so
docker-compose up --build