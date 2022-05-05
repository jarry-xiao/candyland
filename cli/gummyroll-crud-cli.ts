/*
This CLI is meant to help drive testing the backend
*/
import { program } from 'commander';
import log from 'loglevel';
import { PublicKey } from '@solana/web3.js';
import {
    getProvider, initEmptyTree, appendMessage, removeMessage,
    showProof, showAssets, transferMessageOwner, batchInitTree,
    loadBatchInfoFromDir
} from './helpers/crud';
import {
    loadMessages, hashMessages, processLeaves, writeTree,
    writeMetadata, writeProof, loadWalletKey
} from './helpers/utils';
import { buildTree } from '../tests/merkle-tree';
import { mkdirSync } from 'fs';

program.version('0.0.1');
log.setLevel("DEBUG");

function createCommand(commandName) {
    return program.command(commandName)
        .option(
            '-u, --url <string>',
            'RPC url to use',
            undefined
        )
        .option(
            "-k, --keypair <number>",
            "Payer",
            "~/.config/solana/id.json"
        )
}

createCommand("createTree")
    .option(
        "-d, --max-depth <number>",
        'Max depth of tree',
        '14'
    )
    .option(
        "-b, --max-buffer <number>",
        'Maximum # of roots stored (for concurrency)',
        '1024'
    )
    .option(
        "-ak, --authority-keypair <number>",
        "Payer and tree authority",
        "~/.config/solana/id.json"
    )
    .action(async (directory, cmd) => {
        const { url, keypair, authorityKeypair, maxDepth, maxBuffer } = cmd.opts();

        const payer = loadWalletKey(keypair);
        const treeAuthority = loadWalletKey(authorityKeypair);
        const provider = await getProvider(url, payer);
        const tree = await initEmptyTree(
            provider,
            treeAuthority,
            maxDepth,
            maxBuffer
        );

        log.info(`Created empty tree: ${tree.toString()}`);
        log.info(`Max depth ${maxDepth} and max buffer size ${maxBuffer}`);
    });

program
    .command('processMessages')
    .option(
        '-f, --input-file <string>',
        'CSV file containing leaves',
    )
    .option(
        "-d, --max-depth <number>",
        "Max depth of tree",
        "14"
    )
    .option(
        '-o, --out-dir <string>',
        'Output dir',
        'tree-0'
    )
    .action(async (directory, cmd) => {
        const { inputFile, outDir, maxDepth } = cmd.opts();

        log.info("Received messages csv:", inputFile);
        log.info("Writing to outdir:", outDir);
        mkdirSync(outDir, { recursive: true });

        // Create metadata.csv
        const messages = loadMessages(inputFile);
        writeMetadata(messages, outDir);

        // Create changelog.csv
        const hashes = hashMessages(messages);

        // TODO(ngundotra): check that tree height < 22
        const leaves = processLeaves(hashes, maxDepth);

        // Create tree in memory
        const tree = buildTree(leaves);

        // BFS search of tree && write leaves to CSV in 'GM CL' schema
        writeTree(tree, outDir);

        // Write proof info
        writeProof(tree, hashes.length - 1, outDir);
    });

createCommand("batchTree")
    .option(
        "-d, --max-depth <number>",
        'Max depth of tree',
        "14"
    )
    .option(
        "-b, --max-buffer <number>",
        'Maximum # of roots stored (for concurrency)',
        "1024"
    )
    .option(
        "-m, --metadata-uri <string>",
        "URI to metadata csv file",
    )
    .option(
        "-c, --changelog-uri <string>",
        "URI to changelog csv file"
    )
    .option(
        "-m, --dir <string>",
        "Directory to draw metadata from",
        "tree-0"
    )
    .option(
        "-ak, --authority-keypair <number>",
        "Payer and tree authority",
        "~/.config/solana/id.json"
    )
    .action(async (directory, cmd) => {
        const { url, keypair, authorityKeypair, maxDepth, maxBuffer, dir } = cmd.opts();

        const { metadataDbUri, changeLogDbUri, proofInfo } = loadBatchInfoFromDir(dir);
        const payer = loadWalletKey(keypair);
        const treeAuthority = loadWalletKey(authorityKeypair);
        const provider = await getProvider(url, payer);
        const tree = await batchInitTree(
            provider,
            treeAuthority,
            Number(maxDepth),
            Number(maxBuffer),
            metadataDbUri,
            changeLogDbUri,
            proofInfo
        );

        log.info(`Created empty tree: ${tree.toString()}`);
        log.info(`Max depth ${maxDepth} and max buffer size ${maxBuffer}`);
    });


createCommand("appendMessage")
    .description(
        "Uses tree authority to append message"
    )
    .option(
        "-ak, --authority-keypair <number>",
        "Payer and tree authority",
        "~/.config/solana/id.json"
    )
    .option(
        "-m, --message <string>",
        'Message to hash',
        undefined
    )
    .option(
        "-t, --tree <string>",
        "Address of tree",
    )
    .action(async (directory, cmd) => {
        const { url, keypair, authorityKeypair, tree, message } = cmd.opts();
        const payer = loadWalletKey(keypair);
        const treeAuthority = loadWalletKey(authorityKeypair);
        const provider = await getProvider(url, payer);

        await appendMessage(
            provider,
            treeAuthority,
            new PublicKey(tree),
            message,
        );

        console.log(`Wrote "${message}" to a leaf in tree @ ${tree}`);
    });

createCommand("transferMessage")
    .description(
        "Transfers ownership of leaf to a different keypair"
    )
    .option(
        "-ak, --authority-keypair <number>",
        "Payer and tree authority",
        "~/.config/solana/id.json"
    )
    .option(
        '-o, --owner <string>',
        "Owner of leaf containing the message",
        undefined
    )
    .option(
        '-to, --new-owner <string>',
        "New owner of message",
        undefined
    )
    .option(
        "-m, --message <string>",
        'Message to hash',
        undefined
    )
    .option(
        "-n, --index <number>",
        "Index of leaf in tree",
    )
    .option(
        "-t, --tree <string>",
        "Address of tree",
    )
    .option(
        "-p, --proof-url <string>",
        "Proof url",
    )
    .action(async (directory, cmd) => {
        const { owner, index, url, keypair, authorityKeypair, tree, newOwner, message, proofUrl } = cmd.opts();
        const payer = loadWalletKey(keypair);
        const treeAuthority = loadWalletKey(authorityKeypair);
        const provider = await getProvider(url, payer);

        await transferMessageOwner(
            provider,
            proofUrl,
            treeAuthority,
            new PublicKey(tree),
            index,
            owner ? new PublicKey(owner) : treeAuthority.publicKey,
            new PublicKey(newOwner),
            message
        )
    });

createCommand("removeMessage")
    .description(
        "Remove message from tree",
    )
    .option(
        "-ak, --authority-keypair <number>",
        "Payer and tree authority",
        "~/.config/solana/id.json"
    )
    .option(
        '-o, --owner <string>',
        "Owner of leaf containing the message",
        undefined
    )
    .option(
        "-m, --message <string>",
        'Message to hash',
        undefined
    )
    .option(
        "-n, --index <number>",
        "Index of leaf in tree",
    )
    .option(
        "-t, --tree <string>",
        "Address of tree",
    )
    .option(
        "-p, --proof-url <string>",
        "Proof url",
    )
    .action(async (directory, cmd) => {
        const { owner, index, url, keypair, authorityKeypair, tree, message, proofUrl } = cmd.opts();
        const payer = loadWalletKey(keypair);
        const treeAuthority = loadWalletKey(authorityKeypair);
        const provider = await getProvider(url, payer);

        await removeMessage(
            provider,
            proofUrl,
            treeAuthority,
            new PublicKey(tree),
            index,
            owner ? new PublicKey(owner) : treeAuthority.publicKey,
            message
        );
        log.info(`Removed message from index ${index} in tree @ ${tree}`);
    });

createCommand("showProof")
    .option(
        "-n, --index <number>",
        "Index of leaf in tree",
    )
    .option(
        "-t, --tree <string>",
        "Address of tree",
    )
    .option(
        "-p, --proof-url <string>",
        "Proof url",
    )
    .action(async (directory, cmd) => {
        const { index, tree, proofUrl } = cmd.opts();
        await showProof(proofUrl, tree, index);
    })

createCommand("showAssets")
    .option(
        "-o, --owner <string>",
        "Asset owner",
    )
    .option(
        "-p, --proof-url <string>",
        "Proof url",
    )
    .action(async (directory, cmd) => {
        const { owner, proofUrl } = cmd.opts();
        await showAssets(proofUrl, owner);
    })

program.parse(process.argv);
