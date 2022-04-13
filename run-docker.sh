anchor build
cross build --target x86_64-unknown-linux-gnu --package plerkle # Speed up by using cargo remote
cp target/release/libplerkle.so docker-vol/plugin.so # if on mac you ned the lin unknown target in front
cp target/deploy/merkle_wallet.so docker-vol/merkle.so
cp target/deploy/gummyroll.so docker-vol/gummyroll.so
cp target/deploy/gummyroll_crud.so docker-vol/gummyroll_crud.so
cp deps/metaplex-program-library/target/deploy/mpl_token_metadata.so docker-vol/mpl_token_metadata.so
cp deps/solana-program-library/target/deploy/spl_token_2022.so docker-vol/spl_token_2022.so
cp deps/solana-program-library/target/deploy/spl_associated_token_account.so docker-vol/spl_associated_token_account.so