# Batchmint CLI


## How to Use

1. Write messages to CSV

Schema for `hashMessages` follows from GummyrollCrud

```
Owner,Hash
<Pubkey V>,Luke I am your father
<Pubkey L>,noooooo!!!!!
```

2. Convert messages to nodes in the merkle tree

`ts-node index.ts createNodes -f messages.csv -o changelog.csv`

3. Write messages to DB-compatible format

`ts-node index.ts prepareMetadata -f messages.csv -o metadata.csv`

3. Upload changelog & metadata CSVs to arweave

See below
#### Uploading to Arweave
Install `npm i -g @bundlr-network/client`

If fails:
1. `mkdir ~/.npm-global`

2. `npm config set prefix ~/.npm-global`
3. `npm i -g @bundlr-network/client`
4. `export PATH=~/.npm-global:$PATH`


Example of uploading csvs using `bundlr`:

1. `bundlr fund 5000000 -h http://node1.bundlr.network -w ~/.config/solana/id.json -c solana`

2. `bundlr upload changelog.csv -h  https://node1.bundlr.network -w ~/.config/solana/id.json -c solana`

I would have loved to use `arloader`, but honestly it doesn't work all the time. Spamming tx's also causes permanent loss of Sol

##### Arloader example (in case it works again in future)
```
cargo install arloader
arloader upload outfile.csv --with-sol --sol-keypair-path ~/.config/solana/id.json --ar-default-keypair
```

Example Arweave Links:
- changelog.csv "https://arweave.net/iju1AE9qcdKEmGvKXEO41MyHBoeIMh8RQoIsEQDzLu8"
- metadata.csv "https://arweave.net/_1X1bOx0dNhPhbckZ8HsI4ApOXHF8gGV5idnAs5XRXI"
