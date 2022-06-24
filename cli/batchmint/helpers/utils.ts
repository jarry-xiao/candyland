import log from 'loglevel';
import * as fs from 'fs';
import { parse } from 'csv-parse/sync';
import { Buffer } from 'buffer';
import { PublicKey, Keypair, Connection } from '@solana/web3.js';
import { createArrayCsvWriter } from 'csv-writer';
import { bfs, getProofOfLeaf, hash, Tree } from '../../../contracts/tests/merkle-tree';
import { join } from 'path';

export function processLeaves(leaves: Buffer[], maxDepth: number): Buffer[] {
    const leafHashes = leaves.slice();
    const numLeaves = 2 ** maxDepth;
    while (leafHashes.length < numLeaves) {
        leafHashes.push(Buffer.alloc(32));
    }
    return leafHashes;
}


export function writeTree(tree: Tree, outDir: string, fname: string = "changelog.csv") {
    const outFile = join(outDir, fname);
    const writer = createArrayCsvWriter({
        path: outFile,
        header: ['node_idx', 'seq', 'level', 'hash']
    });

    const records = bfs(tree, (treeNode, idx) => {
        return [
            (idx + 1).toString(),
            '0',
            treeNode.level.toString(),
            new PublicKey(treeNode.node).toString(),
        ]
    });

    writer.writeRecords(records);
    log.debug("Wrote tree csv to:", outFile);
}

export function writeMetadata(messages: OwnedMessage[], maxDepth: number, outDir: string, fname: string = "metadata.csv") {
    const outFile = join(outDir, fname);
    const writer = createArrayCsvWriter({
        path: outFile,
        header: ["node_idx", "msg", "owner", "leaf", "revision"]
    });

    const offset = 2 ** maxDepth;
    const records = messages.map((ownedMessage, idx) => {
        return [
            offset + idx,
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
