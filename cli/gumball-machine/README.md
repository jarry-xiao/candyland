# Gumball Machine CLI

## Getting Started

See README.md in the parent directory.

## Usage

This is a CLI for Gumball Machine which generally sits on top of the TypeScript SDK in `candyland/contracts/sdk/gumball-machine`. It provides CLI commands for each core instruction that Gumball Machine supports. Note that before using the CLI, it's critical that you have access to an RPC URL for a network which has the entire Candyland suite deployed (SugarShack not required).

### Key Design Choice

In general, each command corresponds to an instruction in the Gumball Machine smart contract. All accounts that an instruction requires are generally specified as required options for the corresponding command. For more information about the options/arguments of each command you can run `ts-node gumball-machine-cli.ts <command_name> help`.

However, the *arguments* to each instruction are specified in a JSON file. This is because the number of arguments for certain instructions is very large, and would be impractical to be passed directly in the command line. The reason that accounts are not also specified in this JSON file, is to make it easier for the CLI user to identify which accounts they must pass in for a given instruction (by running `ts-node gumball-machine-cli.ts <command_name> help`). For instance, the CLI user does not need to specify the program IDs for `Bubblegum` or `Gummyroll` because they can be pulled directly by the CLI src code. Hence, even though many of the `GumballMachine` instructions require these accounts to be specified, the help command will not display them. Further, for each instruction, some accounts must sign. The user must pass file paths to Keypairs for these accounts. Since the accounts are `requiredOptions`, the `help` output for any given instruction will clearly show which accounts must be passed via file paths and which can just be `Pubkey` seeds. If the accounts were passed in some `keys` field of the `JSON` input then the user would need to discern which `Pubkeys` were required to be manually specified from the docs, and also specfiy some keys as pubkey seeds and others as file paths (signers). This seemed more difficult -- although it does increase the number of options passed via terminal.

It's worth noting that prior to this solution, the arguments for each instruction were specified by the CLI user directly in a TS file, with a hard coded import into `gumball-machine-cli.ts`. This forced the user to work directly with the Solita generated FooBarInstructionArgs type, which was convenient for the src of the CLI, but forced the CLI user a bit too far into the weeds. The JSON solution was chosen to be more canonical, add some input validation, and abstract some implementation details from the CLI user. 

In general, to run the CLI command for some command: `command-name`. You can run the following:

`ts-node gumball-machine-cli.ts command-name -u "<MY-RPC-URL>" -p "<PATH-TO-TXN-PAYER-KEYPAIR>" <keys as given by help> -j "<PATH-TO-JSON-WITH-ARGS">`

Where the given JSON has the following general form:

```
{
  "args": {
    <all fieds found in Solita type: FooBarInstructionArgs>
  },
  "optionals": {
    <optional config parameters, documented here>
  }
}
```

This JSON is then deserialized by the appropriate module in `input-deserialization` into the Solita auto-generated FooBarInstructionArgs type (and potentially with some other information). 

## Docs

### init

Corresponds to `initialize_gumball_machine`. Creates a `GumballMachine` account and the underlying `MerkleRoll` account, paid by `payer`.

Example usage: Creating a GumballMachine taking payment in SOL off of localnet.

`ts-node gumball-machine-cli.ts init -u "http://127.0.0.1:8899" -c "/Users/samorend/Documents/candyland/cli/gumball-machine/creator-keypair.json" -m So11111111111111111111111111111111111111112 -j "./example-input-json/init.json"`

Note: -u specifies the RPC url to be used, -c specifies the path to a keypair which will be the creator of the gumball machine (must sign off on subsequent updates), -m specifies the pubkey of the mint to be used for payments, and -j is the path to the JSON arguments to `initialize_gumball_machine`.

Example `init.json` (note the actual example json directory is also included for reference):

```json
{
  "args": {
    "maxDepth": 3,
    "maxBufferSize": 8,
    "urlBase": "https://arweave.net",
    "nameBase": "GUMBALL",
    "symbol": "GUMBALL",
    "sellerFeeBasisPoints": 100,
    "isMutable": true,
    "retainAuthority": true,
    "encodeMethod": 1,
    "price": 10,
    "goLiveDate": 1234.0,
    "botWallet": "H8DWGKyCSaKtgh1GjyqexVcMe8FPyKQoBhKH79QnhDWx",
    "receiver": "H8DWGKyCSaKtgh1GjyqexVcMe8FPyKQoBhKH79QnhDWx",
    "authority": "Es1EV724ihL7mYzjEiUV1QgcNdwoTpGByLiWSwKFRsAb",
    "collectionKey": null,
    "extensionLen": 28,
    "maxMintSize": 10,
    "maxItems": 250
  },
  "optionals": {
    "canopyDepth": 2
  }
}
```

One thing to note is the optional parameter "canopyDepth". This enables the user to specify some Canopy depth that they want allocated for their merkleRoll account. At a high level, Canopy depth is the number of levels (starting from the root + 1) of the merkle tree to store on chain. Allocating some merkle depth is very helpful in facilitating large drops, and providing some flexibility for interoperable marketplaces etc. 

Note, "optionals" can be completely omitted, or can be specified as empty and the command will still succeed with a canopy depth of 0.

Take note of the output of the command, as it will give the pubkeys of the created accounts, which are needed for subsequent commands.

Example output:
```
Created Gumball Machine Pubkey: 4SLVDrLyadq6rDb3sKiDR2nX99BiGvXME51Z8B3sPCrE
Created Merkle Roll Publickey: FFsMEokRrRNmaWXoXeud42A27UNXq7qzfLCDbrosyoC4
```

### add-config-lines

Corresponds to `add_config_lines`. Adds config lines to the specific GumballMachine account to enable future minting.

Example usage: adding config lines to a GumballMachine on localnet.

`ts-node gumball-machine-cli.ts add-config-lines -u "http://127.0.0.1:8899" -a "/Users/samorend/Documents/candyland/cli/gumball-machine/creator-keypair.json" -g 4MMxH3v4h9MsWANh9NDVSRNa4zmEgXDRqb9y7nkKzrmb -j "./example-input-json/add-config-lines.json"`

Note: -a is the path to a keypair of the authority of the GumballMachine. -g is the pubkey of the GumballMachine itself.

Example `add-config-lines.json`:

```json
{
  "args": {
    "newConfigLines": [
      "uluvnpwncgchwnbqfpbtdlcpdthc",
      "aauvnpwncgchwnbqfpbtdlcpdthc"
    ]
  },
  "optionals": {}
}
```

This command does not currently support any optionals, the optionals field can be omitted entirely.

### update-config-lines

Corresponds to `update_config_lines`. Updates the contents of already added config lines.

Example usage: updating config lines of a GumballMachine on localnet.

`ts-node gumball-machine-cli.ts update-config-lines -u "http://127.0.0.1:8899" -a "/Users/samorend/Documents/candyland/cli/gumball-machine/creator-keypair.json" -g 4MMxH3v4h9MsWANh9NDVSRNa4zmEgXDRqb9y7nkKzrmb -j "./example-input-json/update-config-lines.json"`

Example `update-config-lines.json`:

```json
{
  "args": {
    "startingLine": 0,
    "newConfigLines": [
      "uluvnpwncgchwnbqfpbtdlcpdthc",
      "aauvnpwncgchwnbqfpbtdlcpdthc"
    ]
  },
  "optionals": {}
}
```

Starts overwriting config line data from `startingLine`.

This command does not currently support any optionals, the optionals field can be omitted entirely.

### update-header-metadata

Corresponds to `update_header_metadata`. Update the `GumballMachineHeader` for a `GumballMachine`.

Example usage: updating header properties of a GumballMachine on localnet.

`ts-node gumball-machine-cli.ts update-header-metadata -u "http://127.0.0.1:8899" -a "/Users/samorend/Documents/candyland/cli/gumball-machine/creator-keypair.json" -g 4MMxH3v4h9MsWANh9NDVSRNa4zmEgXDRqb9y7nkKzrmb -j "./example-input-json/update-header-metadata.json"`

Example `update-header-metadata.json`:

```json
{
  "args": {
    "urlBase": "https://arweave.net",
    "nameBase": "GUMBALL",
    "symbol": "GUMBALL",
    "sellerFeeBasisPoints": 100,
    "isMutable": true,
    "retainAuthority": true,
    "encodeMethod": 1,
    "price": 10,
    "goLiveDate": 1234.0,
    "botWallet": "CwKozhKgAEkxHhJ6AwBn6faFstKWmoFB4JYz18RyxExz",
    "authority": "Es1EV724ihL7mYzjEiUV1QgcNdwoTpGByLiWSwKFRsAb",
    "maxMintSize": 10
  },
  "optionals": {}
}
```

This command does not currently support any optionals, the optionals field can be omitted entirely.

### destroy

Corresponds to `destroy`. Destroys a gumball machine, reclaim rent.

Example usage: destroy a `GumballMachine` on localnet.

`ts-node gumball-machine-cli.ts destroy -u "http://127.0.0.1:8899" -a "/Users/samorend/Documents/candyland/cli/gumball-machine/creator-keypair.json" -g 4SLVDrLyadq6rDb3sKiDR2nX99BiGvXME51Z8B3sPCrE`

Note: destroy does not have any arguments, so -j is not required (or supported) for this command.

### dispense-nft-sol

Corresponds to `dispense_nft_sol` on-chain. Dispenses a certain number of NFTs in exchange for payment in SOL.

Example usage: purchase an NFT with SOL from a `GumballMachine` on localnet.

`ts-node gumball-machine-cli.ts dispense-nft-sol -u "http://127.0.0.1:8899" -g kxTL1hAenEKiZ7vCbRTtH2VBwWDmSntThS6x5UZsPAQ -j "./example-input-json/dispense-nft-sol.json" -m DT9ejTkku2XrLJoDeNgTY3m9835paesjxqbN2nCzPqgV -r CwKozhKgAEkxHhJ6AwBn6faFstKWmoFB4JYz18RyxExz`

Note: depending on the price of the NFT the receiver might receive such a small payout that it would not be rent exempt and the payout fails. If this occurs, simply airdrop some funds to the receiver pubkey in advance. Also observe that -p is omitted here, because it defaults to the Solana default keypair, which is the account that will receive the compressed NFT.

Example `dispense-nft-sol.json`:

```json
{
  "args": {
    "numItems": 1
  },
  "optionals": {}
}
```

This command does not currently support any optionals, the optionals field can be omitted entirely.

### dispense-nft-token

Corresponds to `dispense_nft_token` on-chain. Dispenses a certain number of NFTs in exchange for payment in some SPL Token.

Example usage: purchase an NFT with Tokens from a `GumballMachine` on localnet.

`ts-node gumball-machine-cli.ts dispense-nft-token -u "http://127.0.0.1:8899" -g GvpNy2Pw364CYhXk9ws3DMNWzTRe12LUyYP7MpJx7wkx -j "./example-input-json/dispense-nft-token.json" -m 6rb7AQgVk7BLjoTnD4Hmjo78if2fy7fTRnB6TGkaMxS6 -r H8DWGKyCSaKtgh1GjyqexVcMe8FPyKQoBhKH79QnhDWx -t DKdyWVKLy2XPbSHMXaGBHkFAg59x7V4dRwEvjUrC57me`

Note: -r is the pubkey of the account that the `GumballMachine` header has specified as the receiver. -t is the payer's token account that is used to make the payment.

Example `dispense-nft-token.json`:

```json
{
  "args": {
    "numItems": 1
  },
  "optionals": {}
}
```

This command does not currently support any optionals, the optionals field can be omitted entirely.
