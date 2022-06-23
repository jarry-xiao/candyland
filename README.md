# 🍬 Candyland 🍬

Smart contracts and indexing services necessary to migrate the Solana ecosystem to a 10,000x cheaper NFT standard.

```mermaid
graph TD;
    subgraph contracts
        gummyroll-->concurrent-merkle-tree;
        bubblegum-->gummyroll;
        gumball-machine-->bubblegum;
        gumdrop-->bubblegum;
    end
    subgraph indexer
        plerkle-->solana-geyser-plugin;
        nft-ingester-->|messenger|plerkle;
        nft-api-->nft-ingester;
    end
```

# Smart Contracts

| Package | Description | Docs | Audit | Program Id |
| :-- | :-- | :--| :-- | :-- |
| `gummyroll` | On-chain merkle tree that supports concurrent writes | tbd | tbd | `GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD` |
| `bubblegum` | Token transfer and metadata functionality built on top of gummyroll | tbd | tbd | `BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY` |
| `gumball-machine` | Candy machine built for bubblegum | tbd | tbd | `GBALLoMcmimUutWvtNdFFGH5oguS7ghUUV6toQPppuTW` | 
| `gumdrop` | Forked version of mpl gumdrop, with support for bubblegum | tbd | tbd | `gdrpGjVffourzkdDRrQmySw4aTHr8a3xmQzzxSwFD1a` | 
| (deprecated) `gummyroll-CRUD` | an example messaging (CRUD) program built on top of gummyroll | deprecated | deprecated | deprecated |

### Gummyroll - Merkle Tree 

Merkle tree root stores information of its leaves. 
We store a buffer of proof-like changelogs on-chain that allow multiple proof-based writes to succeed within the same slot.
This is accomplished by fast-forwarding out-of-date or possibly invalid proofs based upon the information in the changelogs.


Information about max tree height, maximum transaction size, and other constraints can be found in `tests/txLength.ts`.

##### Note on hashing:
It's industry standard to lexicographically sort inner nodes when hashing up the tree. However `gummyroll` does not implement this. Since indices are needed to find the intersection for the changelog array, we implement hashing using an index to order the nodes.

### Bubblegum - NFTs in Merkle Trees

Supports decompressing `bubblegum` NFTs into either `Tokenkeg` tokens or `Token22` tokens.
The benefit of decompressing a `bubblegum` NFT is that normal tokens can be moved into a custodial wallet and freely transferred
without relying on RPC nodes to serve your NFT data from an off-chain database.

### Gumball machine - Candy machine for NFT drops
For more information on candy machine: `https://docs.metaplex.com/candy-machine-v2/introduction`

### Gumdrop - Airdrop compressed NFTs
Copied from here: `https://github.com/metaplex-foundation/metaplex-program-library/tree/master/gumdrop`

Additions to the MPL gumdrop: 
- `new_distributor_compressed` ix (needed to setup `claim_bubblegum`)
- `claim_bubblegum` ix (needed to claim NFTs into compressed tree)

# Indexer

This is the bread and butter of this project. Gummyroll relies on RPC indexers to store merkle tree leaf data off-chain. 

| Portions | Description | Docs |
| :------- | :------- | :--- |
| `nft_ingester` | Service to ingest compressed NFT events from logs and insert into postgres database | tbd |
| `nft_api` | REST api to serve proofs and other information from postgres database. Eventually will become JSON RPC api. | tbd |
| `plerkle` | Generalized geyser plugin to store regular and compressed NFT information | tbd |
| `plerkle_serialization` | Flatbuffer schemas for optimally transporting geyser plugin information | tbd |
| `messenger` | Traits needed to generalize messaging bus for NFT related indexing | tbd |

## Getting Started
```
cd candyland/
git submodule update --init --recursive
yarn install
cd contracts/
anchor build
cd ..
docker compose up --build --force-recreate
```

#### In another terminal:
```
cd candyland/contracts/
yarn
yarn run ts-mocha -t 1000000 tests/bubblegum-test.ts
yarn run ts-mocha -t 1000000 tests/continuous_gummyroll-test.ts
```

## Running Tests

`anchor test` will run tests.

Note: all tests will pass locally except `continuous_gummyroll-test.ts` which requires a server to point it's connection to.

If you get an error referencing a missing path in `deps/anchor/tests`, then cd into that test repo and build it.

For example: `cd deps/anchor/tests/misc && anchor build`

If tests are failing by timing out, then this likely means that certain programs are not loaded in the local validator.
This is remedied by adding the program & address to a `[[test.genesis]]` entry in `Anchor.toml`.
You can tell if this is the issue by turning `skipPreflight` to `false`. Simulation error will show programId not found.

## Generating typesafe query types
Follow this guide Follow this guide https://www.sea-ql.org/SeaORM/docs/generate-entity/sea-orm-cli for setup
```
cd digital_asset_types
cargo install sea-orm-cli
```

make sure you `docker compose up db`. and have a ENV var setup `DATABASE_URL=postgres://solana:solana@localhost/solana`

`
sea-orm-cli generate entity -o entity/src --database-url $DATABASE_URL --expanded-format
`

