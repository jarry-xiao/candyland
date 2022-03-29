anchor build
cp deps/accountsdb-postgres/target/x86_64-unknown-linux-gnu/debug/libsolana_accountsdb_plugin_postgres.so docker/psl.so
cp plerkle/target/x86_64-unknown-linux-gnu/debug/libplerkle.so docker/plugin.so
cp target/deploy/merkle_wallet.so docker/merkle.so
cp target/deploy/gummyroll.so docker/gummyroll.so
cp deps/metaplex-program-library/target/deploy/mpl_token_metadata.so docker/mpl_token_metadata.so
cp deps/solana-program-library/target/deploy/spl_token_2022.so docker/spl_token_2022.so
cp deps/solana-program-library/target/deploy/spl_associated_token_account.so docker/spl_associated_token_account.so
docker-compose build solana