#!/usr/bin/env bash
# @dev: if you introduce a new SDK dependency i.e. importing the sugar-shack SDK to CLI
#       plz update this script so all devs can easily get setup

rm -rf cli/node_modules

rm -rf contracts/node_modules

rm -rf contracts/sdk/prototype-indexer/node_modules

rm -rf contracts/sdk/utils/node_modules
cd contracts/sdk/utils/
yarn
yarn run build
yarn link
cd ../../../

rm -rf contracts/sdk/gummyroll/node_modules
cd contracts/sdk/gummyroll/
yarn link "@sorend-solana/utils"
yarn
yarn run build
yarn link
cd ../../../

rm -rf contracts/sdk/bubblegum/node_modules
cd contracts/sdk/bubblegum/
yarn link "@sorend-solana/utils"
yarn link "@sorend-solana/gummyroll"
yarn
yarn run build
yarn link
cd ../../../

rm -rf contracts/sdk/gumball-machine/node_modules
cd contracts/sdk/gumball-machine/
yarn link "@sorend-solana/utils"
yarn link "@sorend-solana/gummyroll"
yarn link "@sorend-solana/bubblegum"
yarn
yarn run build
yarn link
cd ../../../

rm -rf contracts/sdk/sugar-shack/node_modules
cd contracts/sdk/sugar-shack/
yarn
yarn run build
yarn link
cd ../../../

cd cli
yarn link "@sorend-solana/utils"
yarn link "@sorend-solana/gummyroll"
yarn link "@sorend-solana/gumball-machine"
yarn
cd ../

cd contracts
yarn
cd ../

cd contracts/tests
yarn link "@sorend-solana/utils"
yarn link "@sorend-solana/gummyroll"
yarn link "@sorend-solana/bubblegum"
yarn link "@sorend-solana/gumball-machine"
yarn link "@sorend-solana/sugar-shack"
yarn
cd ../../

cd contracts/sdk/prototype-indexer
yarn link "@sorend-solana/utils"
yarn link "@sorend-solana/gummyroll"
yarn link "@sorend-solana/bubblegum"
yarn link "@sorend-solana/gumball-machine"
yarn
cd ../../

