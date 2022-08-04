#!/usr/bin/env bash
# @notice: Used to unlink all public SDKs: Utils, Gummyroll, Bubblegum, Gumball Machine, Sugar Shack and install + use these dependencies from the registry.

cd contracts/sdk/utils/
yarn unlink
cd ../../../

rm -rf contracts/sdk/gummyroll/node_modules
cd contracts/sdk/gummyroll/
yarn
yarn run build
yarn unlink
cd ../../../

rm -rf contracts/sdk/bubblegum/node_modules
cd contracts/sdk/bubblegum/
yarn
yarn run build
yarn unlink
cd ../../../

rm -rf contracts/sdk/gumball-machine/node_modules
cd contracts/sdk/gumball-machine/
yarn
yarn run build
yarn unlink
cd ../../../

cd contracts/sdk/sugar-shack/
yarn unlink
cd ../../../

rm -rf cli/node_modules
cd cli
yarn
cd ../

rm -rf contracts/tests/node_modules
cd contracts/tests
yarn
cd ../../

rm -rf contracts/sdk/indexer/node_modules
cd contracts/sdk/indexer
yarn
cd ../../

