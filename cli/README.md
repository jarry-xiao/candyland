# Batchmint CLI Readme

## How to Use

1. Write messages to CSV

Schema for `processMessages` follows from GummyrollCrud

```
Owner,Hash
<Pubkey V>,Luke I am your father
<Pubkey L>,noooooo!!!!!
```

2. Process these messages to be ready for batch mint instruction

`ts-node gummyroll-crud-cli.ts processMessages -f messages.csv -m tree-0`

3. Upload metadata & changelog csv to Arweave

- (required to init bundlr node) `bundlr fund 5000000 -h http://node1.bundlr.network -w ~/.config/solana/id.json -c solana` 
- `bundlr upload tree-0/metadata.csv -h  https://node1.bundlr.network -w ~/.config/solana/id.json -c solana`
- `bundlr upload tree-0/changelog.csv -h  https://node1.bundlr.network -w ~/.config/solana/id.json -c solana`

3. Write arweave links manually to `tree-0/upload.json`

Example `tree-0/upload.json`:
```json
{
    "changelogUri": "https://arweave.net/iju1AE9qcdKEmGvKXEO41MyHBoeIMh8RQoIsEQDzLu8",
    "metadataUri": "https://arweave.net/1c1-QMoJ-G2mfFPJ7qNqpHlj7KPDV-XlZ0DOkZbIZPA"
}
```

4. Batch mint tree!

`ts-node gummyroll-crud-cli.ts batchTree -m tree-0 -u "RPC_URL"`

5. See uploaded assets at `/owner/<owner address>/assets`

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

