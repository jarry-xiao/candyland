import { Keypair, PublicKey } from '@solana/web3.js';
import { program } from 'commander';
import log from 'loglevel';
import { buildTree } from '../tests/merkle-tree';
import { writeHashes, loadMessages, hashMessages, loadLeaves, writeTree, writeMetadata } from './helpers/utils';

program.version('0.0.1');
log.setLevel('DEBUG');

program
    .command('createNodes')
    .option(
        '-f, --input-file <string>',
        'CSV file containing leaves',
    )
    .option(
        '-o, --out-file <string>',
        'Output CSV file, ready to be uploaded to arweave',
        'outfile.csv'
    )
    .option(
        '-d, --max-depth <number>',
        'Max depth of the tree to be supported',
        '14'
    )
    .action(async (directory, cmd) => {
        const { inputFile, outFile, maxDepth } = cmd.opts();

        log.info("Received input file:", inputFile);
        log.info("Writing to file:", outFile);
        log.info("depth is:", maxDepth);
        log.info('\n');

        // Load in leaves, up to max depth
        const leaves = loadLeaves(inputFile, maxDepth);

        // Create tree in memory
        const tree = buildTree(leaves);

        // BFS search of tree && write leaves to CSV in 'GM CL' schema
        writeTree(tree, outFile);
    });

program.command('hashMessages')
    .option(
        '-f, --input-file <string>',
        'CSV file containing owner,message columns',
    )
    .option(
        '-o, --out-file <string>',
        'Output CSV to be used in batchMint',
        'test-input.csv'
    )
    .action(async (directory, cmd) => {
        const { inputFile, outFile } = cmd.opts();
        const messages = loadMessages(inputFile);
        const hashes = hashMessages(messages);
        writeHashes(hashes, outFile);
    });

program.command('prepareMetadata')
    .option(
        '-f, --input-file <string>',
        'CSV file containing owner,message columns',
    )
    .option(
        '-o, --out-file <string>',
        'Output CSV to be used in batchMint',
        'metadata.csv'
    )
    .action(async (directory, cmd) => {
        const { inputFile, outFile, pubkey } = cmd.opts();

        let treeId: PublicKey;
        if (pubkey == undefined) {
            treeId = Keypair.generate().publicKey;
        } else {
            treeId = new PublicKey(pubkey);
        }

        const messages = loadMessages(inputFile);
        writeMetadata(messages, outFile);
    });


program.parse(process.argv);
