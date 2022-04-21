import { Keypair, PublicKey } from '@solana/web3.js';
import { program } from 'commander';
import log from 'loglevel';
import { buildTree, hashLeaves, Tree } from '../tests/merkle-tree';
import { writeHashes, loadMessages, hashMessages, loadLeaves, writeTree } from './utils';

program.version('0.0.1');
log.setLevel('DEBUG');

program
    .command('batchMint')
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
    .option(
        '-p, --pubkey <string>',
        'Pubkey of the tree',
        undefined
    )
    .action(async (directory, cmd) => {
        const { inputFile, outFile, maxDepth, pubkey } = cmd.opts();

        let treeId: PublicKey;
        if (pubkey == undefined) {
            treeId = Keypair.generate().publicKey;
        } else {
            treeId = new PublicKey(pubkey);
        }

        log.info("Received input file:", inputFile);
        log.info("Writing to file:", outFile);
        log.info("Tree id is:", treeId.toString());
        log.info("depth is:", maxDepth);
        log.info('\n');

        // Load in leaves, up to max depth
        const leaves = loadLeaves(inputFile, maxDepth);

        // Create tree in memory
        const tree = buildTree(leaves);

        // BFS search of tree && write leaves to CSV in 'GM CL' schema
        writeTree(tree, treeId, outFile);
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

program.parse(process.argv);
