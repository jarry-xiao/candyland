import { Command } from 'commander';
import log from 'loglevel';
import {
    PublicKey,
    Keypair,
    SystemProgram,
    Transaction,
    Connection as web3Connection,
    LAMPORTS_PER_SOL,
    ComputeBudgetInstruction,
    ComputeBudgetProgram
} from "@solana/web3.js";
import {
    getProvider, loadWalletKey
} from '../helpers/utils';
import {
    createDispenseNFTForSolIx,
    createDispenseNFTForTokensIx,
    createInitializeGumballMachineIxs,
    createAddConfigLinesInstruction,
    createUpdateConfigLinesInstruction,
    createUpdateHeaderMetadataInstruction,
    createDestroyInstruction,
    initializeGumballMachineIndices,
} from "../../contracts/sdk/gumball-machine";
import { Program, Provider } from '@project-serum/anchor';
import {
    deserializeInitJson
} from "./input-deserialization/initializeGumballMachine";
import {
    deserializeAddConfigLinesJson
} from "./input-deserialization/addConfigLines";
import {
    deserializeUpdateConfigLinesJson
} from "./input-deserialization/updateConfigLines";
import {
    deserializeUpdateHeaderMetadataJson
} from "./input-deserialization/updateHeaderMetadata";
import {
    deserializeDispenseNFTSolJson
} from "./input-deserialization/dispenseNFTForSol";
import {
    deserializeDispenseNFTTokensJson
} from "./input-deserialization/dispenseNFTForTokens";
import {
    execute
} from "../../contracts/sdk/utils";
import {
    readFileSync
} from "fs";
import {
    resolve
} from 'path';
import { deserializeInitIndicesJson } from './input-deserialization/initIndices';

const program = new Command();
program
    .name('gumball-machine-cli')
    .description('CLI for the Gumball Machine SDK')
    .version('0.0.1');

log.setLevel("DEBUG");

function createCommand(commandName) {
    return program.command(commandName)
        .option(
            '-u, --url <string>',
            'RPC url to use',
            undefined
        )
        .option(
            "-p, --payer-keypair-path <number>",
            "Payer",
            "~/.config/solana/id.json"
        )
}

createCommand("init")
    .description("Initialize a new gumball machine from scratch. Creates merkleRoll and gumballMachine accounts.")
    .requiredOption(
        "-c, --creator-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-m, --mint-pubkey <string>",
        'Mint of tokens used for payments for GumballMachine'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, creatorKeypairPath, mintPubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const mintPublicKey = new PublicKey(mintPubkey);
        const provider = await getProvider(url, payerKeypair);
        const creatorKeypair = loadWalletKey(creatorKeypairPath);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const [gumballMachineInitArgs, gumballMachineAcctSize, merkleRollAcctSize] = deserializeInitJson(inputObject);

        const gumballMachineKeypair = Keypair.generate();
        log.info(`Created Gumball Machine Pubkey: ${gumballMachineKeypair.publicKey.toString()}`);

        const merkleRollKeypair = Keypair.generate();
        log.info(`Created Merkle Roll Publickey: ${merkleRollKeypair.publicKey.toString()}`);

        // Creator funds creation of gumballMachine and merkleRoll accounts
        await provider.connection.confirmTransaction(
            await provider.connection.requestAirdrop(
                creatorKeypair.publicKey,
                75 * LAMPORTS_PER_SOL
            ),
            "confirmed"
        );

        const initializeGumballMachineInstrs =
            await createInitializeGumballMachineIxs(
                creatorKeypair.publicKey,
                gumballMachineKeypair.publicKey,
                gumballMachineAcctSize,
                merkleRollKeypair.publicKey,
                merkleRollAcctSize,
                gumballMachineInitArgs,
                mintPublicKey,
                provider.connection
            );
        const txId = await execute(provider, initializeGumballMachineInstrs, [creatorKeypair, gumballMachineKeypair, merkleRollKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("init-indices")
    .description("Initialize the NFT indices for the gumball machine. This command may execute multiple transactions.")
    .requiredOption(
        "-a, --authority-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, authorityKeypairPath, gumballMachinePubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const authorityKeypair = loadWalletKey(authorityKeypairPath);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const initIndicesArgs = deserializeInitIndicesJson(inputObject);

        const provider = await getProvider(url, payerKeypair);

        await initializeGumballMachineIndices(provider, initIndicesArgs.maxItems, authorityKeypair, gumballMachinePublicKey, true);
    });

createCommand("add-config-lines")
    .description("Add new config lines to a gumball machine")
    .requiredOption(
        "-a, --authority-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, authorityKeypairPath, gumballMachinePubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const authorityKeypair = loadWalletKey(authorityKeypairPath);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const configLinesToAdd = deserializeAddConfigLinesJson(inputObject);

        const provider = await getProvider(url, payerKeypair);

        const addConfigLinesInstr =
            await createAddConfigLinesInstruction(
                {
                    gumballMachine: gumballMachinePublicKey,
                    authority: authorityKeypair.publicKey,
                },
                configLinesToAdd
            );
        const txId = await execute(provider, [addConfigLinesInstr], [authorityKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("update-config-lines")
    .description("Update config lines of a gumball machine")
    .requiredOption(
        "-a, --authority-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, authorityKeypairPath, gumballMachinePubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const authorityKeypair = loadWalletKey(authorityKeypairPath);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const updateConfigLinesArgs = deserializeUpdateConfigLinesJson(inputObject);

        const provider = await getProvider(url, payerKeypair);

        const updateConfigLinesInstr =
            await createUpdateConfigLinesInstruction(
                {
                    gumballMachine: gumballMachinePublicKey,
                    authority: authorityKeypair.publicKey,
                },
                updateConfigLinesArgs
            );
        const txId = await execute(provider, [updateConfigLinesInstr], [authorityKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("update-header-metadata")
    .description("Update the header metadata of a gumball machine")
    .requiredOption(
        "-a, --authority-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, authorityKeypairPath, gumballMachinePubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const authorityKeypair = loadWalletKey(authorityKeypairPath);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const updateGumballMachineHeaderArgs = deserializeUpdateHeaderMetadataJson(inputObject);

        const provider = await getProvider(url, payerKeypair);

        const updateHeaderMetadataInstr =
            await createUpdateHeaderMetadataInstruction(
                {
                    gumballMachine: gumballMachinePublicKey,
                    authority: authorityKeypair.publicKey,
                },
                updateGumballMachineHeaderArgs
            );
        const txId = await execute(provider, [updateHeaderMetadataInstr], [authorityKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("destroy")
    .description("Destroy a gumball machine and recover rent")
    .requiredOption(
        "-a, --authority-keypair-path <string>",
        'Path to gumball machine creator keypair'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .action(async (options) => {
        const { url, payerKeypairPath, authorityKeypairPath, gumballMachinePubkey } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const authorityKeypair = loadWalletKey(authorityKeypairPath);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);

        const provider = await getProvider(url, payerKeypair);

        const destroyInstr =
            await createDestroyInstruction(
                {
                    gumballMachine: gumballMachinePublicKey,
                    authority: authorityKeypair.publicKey,
                }
            );
        const txId = await execute(provider, [destroyInstr], [authorityKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("dispense-nft-sol")
    .description("Purchase compressed NFTs from a Gumball Machine with SOL")
    .requiredOption(
        "-r, --receiver-pubkey <string>",
        'Pubkey of the fee receiver for the gumball machine'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-m, --merkle-roll-pubkey <string>",
        'Pubkey of the merkle roll account'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, receiverPubkey, gumballMachinePubkey, merkleRollPubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const receiverPublicKey = new PublicKey(receiverPubkey);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);
        const merkleRollPublicKey = new PublicKey(merkleRollPubkey);

        const provider = await getProvider(url, payerKeypair);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const dispenseNFTSolArgs = deserializeDispenseNFTSolJson(inputObject);

        const dispenseNFTForSolIx =
            await createDispenseNFTForSolIx(
                dispenseNFTSolArgs,
                payerKeypair.publicKey,
                receiverPublicKey,
                gumballMachinePublicKey,
                merkleRollPublicKey
            );
        const txId = await execute(provider, [dispenseNFTForSolIx], [payerKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

createCommand("dispense-nft-token")
    .description("Purchase compressed NFTs from a Gumball Machine with SPL Tokens")
    .requiredOption(
        "-t, --payer-tokens-pubkey <string>",
        'Pubkey of the associated token account for the payers tokens'
    )
    .requiredOption(
        "-r, --receiver-pubkey <string>",
        'Pubkey of the fee receiver for the gumball machine'
    )
    .requiredOption(
        "-g, --gumball-machine-pubkey <string>",
        'Public key of the gumball machine'
    )
    .requiredOption(
        "-m, --merkle-roll-pubkey <string>",
        'Pubkey of the merkle roll account'
    )
    .requiredOption(
        "-j, --json-config-filepath <string>",
        "File path to JSON file with initialization args"
    )
    .action(async (options) => {
        const { url, payerKeypairPath, receiverPubkey, gumballMachinePubkey, merkleRollPubkey, payerTokensPubkey, jsonConfigFilepath } = options;

        const payerKeypair = loadWalletKey(payerKeypairPath);
        const payerTokensPublicKey = new PublicKey(payerTokensPubkey);
        const receiverPublicKey = new PublicKey(receiverPubkey);
        const gumballMachinePublicKey = new PublicKey(gumballMachinePubkey);
        const merkleRollPublicKey = new PublicKey(merkleRollPubkey);

        const provider = await getProvider(url, payerKeypair);

        const inputObject = JSON.parse(readFileSync(resolve(__dirname, jsonConfigFilepath)).toString());
        const dispenseNFTTokenArgs = deserializeDispenseNFTTokensJson(inputObject);

        const dispenseNFTForTokensIx =
            await createDispenseNFTForTokensIx(
                dispenseNFTTokenArgs,
                payerKeypair.publicKey,
                payerTokensPublicKey,
                receiverPublicKey,
                gumballMachinePublicKey,
                merkleRollPublicKey
            );
        const txId = await execute(provider, [dispenseNFTForTokensIx], [payerKeypair], false, true);
        log.info(`TX Completed Successfully: ${txId}`);
    });

program.parse(process.argv);
