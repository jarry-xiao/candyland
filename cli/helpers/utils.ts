import log from 'loglevel';
import * as fs from 'fs';
import { parse } from 'csv-parse/sync';
import { Buffer } from 'buffer';
import { PublicKey, Keypair, Connection } from '@solana/web3.js';
import { createArrayCsvWriter } from 'csv-writer';
import { bfs, getProofOfLeaf, hash, Tree } from '../../tests/merkle-tree';
import { join } from 'path';

// type IndexedLeaves = {
//     leafIndex: string,
//     hash: string,
// };

/**
 * Takes a sorted list of leaf hashes from the input file
 * and throws if a precondition is violated
 * @param leaves 
 */
// function validateLeaves(leaves: IndexedLeaves[]) {
//     // Check de-duped
//     leaves.map((leaf, index) => {
//         if (Number(leaf.leafIndex) != index) {
//             throw new Error(`leafIndex mismatch for ${index}th leaf which incorrectly has 'leafIndex' set to ${leaf.leafIndex}`);
//         }
//         try {
//             const _pubkey = new PublicKey(leaf.hash);
//         } catch (e) {
//             throw new Error(
//                 `Could not create pubkey from the bytes of hash-- index: ${index}, bytes: ${leaf.hash}, leafIndex: ${leaf.leafIndex}\n${e}`
//             );
//         }
//     });

//     // Check that # of leaves matches up
//     if (Number(leaves[leaves.length - 1].leafIndex) != leaves.length - 1) {
//         throw new Error("Unable to proceed, final # of leaf_indices != # of hashes provided");
//     }
// }

export function processLeaves(leaves: Buffer[], maxDepth: number): Buffer[] {
    const leafHashes = leaves.slice();
    // leaves = leaves.sort((left, right) => Number(left.leafIndex) - Number(right.leafIndex));

    // validateLeaves(leaves);

    // leaves.map((leaf) => {
    //     leafHashes.push(new PublicKey(leaf).toBuffer());
    // });
    // const height = Math.ceil(Math.log2(leaves.length))
    const numLeaves = 2 ** maxDepth;
    while (leafHashes.length < numLeaves) {
        leafHashes.push(Buffer.alloc(32));
    }
    return leafHashes;
}

// export function loadLeaves(inputFile: string, maxDepth: number) {
//     const leaves = parse(fs.readFileSync(inputFile).toString(), {
//         columns: true,
//         skipEmptyLines: true,
//     });
//     log.debug(`Loaded ${leaves.length} leaves from ${inputFile}`);
//     return processLeaves(leaves, maxDepth);
// }

/**
 * Do BFS from the tree root down to leaves & write to outFile
 */
export function writeTree(tree: Tree, outDir: string, fname: string = "changelog.csv") {
    const outFile = join(outDir, fname);
    const writer = createArrayCsvWriter({
        path: outFile,
        header: ['node_idx', 'seq', 'level', 'hash']
    });

    log.debug("doing bfs on a tree");
    const records = bfs(tree, (treeNode, idx) => {
        return [
            (idx + 1).toString(),
            '0',
            treeNode.level.toString(),
            new PublicKey(treeNode.node).toString(),
        ]
    });

    log.debug(records[0], records[records.length - 1]);
    writer.writeRecords(records);
    log.debug("Wrote tree csv to:", outFile);
}

export function writeMetadata(messages: OwnedMessage[], outDir: string, fname: string = "metadata.csv") {
    const outFile = join(outDir, fname);
    const writer = createArrayCsvWriter({
        path: outFile,
        header: ["msg", "owner", "leaf", "revision"]
    });

    const records = messages.map((ownedMessage) => {
        return [
            ownedMessage.message,
            ownedMessage.owner,
            new PublicKey(hashOwnedMessage(ownedMessage)).toString(),
            0
        ]
    })
    writer.writeRecords(records);
    log.debug("Wrote metadata csv to:", outFile);
}

type OwnedMessage = {
    owner: String,
    message: String,
}

export function loadMessages(inputFile: string): OwnedMessage[] {
    const messages = parse(fs.readFileSync(inputFile).toString(), {
        columns: true,
        skipEmptyLines: true,
    });
    return messages;
}

function hashOwnedMessage(ownedMessage: OwnedMessage): Buffer {
    return hash(new PublicKey(ownedMessage.owner).toBuffer(), Buffer.from(ownedMessage.message));
}

export function hashMessages(messages: OwnedMessage[]): Buffer[] {
    return messages.map((ownedMessage) => {
        return hashOwnedMessage(ownedMessage)
    });
}

// /**
//  * Writes minimal changelog csv (just leaves) to csv
//  * @param messages 
//  * @param outDir 
//  * @param fname 
//  */
// export function writeChangelog(messages: Buffer[], outDir: string, fname: string = "changelog.csv") {
//     const writer = createArrayCsvWriter({
//         path: join(outDir, fname),
//         header: ['leafIndex', 'hash']
//     });
//     const records = messages.map((buffer, index) => {
//         return [
//             index.toString(),
//             new PublicKey(buffer).toString(),
//         ]
//     });
//     log.debug("Records", records);
//     writer.writeRecords(records as any[]);
// }

export function writeProof(tree: Tree, rightMostIndex: number, outDir: string, fname: string = "proof.json") {
    const outFile = join(outDir, fname);
    const proof = getProofOfLeaf(tree, rightMostIndex);
    const proofInfo = {
        proof: proof.map((node) => new PublicKey(node.node).toString()),
        leaf: new PublicKey(tree.leaves[rightMostIndex].node).toString(),
        root: new PublicKey(tree.root).toString(),
        index: rightMostIndex,
    }
    fs.writeFileSync(
        outFile,
        JSON.stringify(proofInfo, undefined, 2)
    );
    log.info("Wrote proof json to:", outFile);
}

export function loadWalletKey(keypair: string): Keypair {
    if (!keypair || keypair == '') {
        throw new Error('Keypair is required!');
    }
    keypair = keypair.replace("~", process.env.HOME);
    const loaded = Keypair.fromSecretKey(
        new Uint8Array(JSON.parse(fs.readFileSync(keypair).toString())),
    );
    return loaded;
}

export async function confirmTxOrThrow(connection: Connection, txId: string) {
    const result = await connection.confirmTransaction(txId, "confirmed");
    if (result.value.err) {
        throw new Error(`Failed to execute transaction: ${result.value.err.toString()}`);
    }
}
