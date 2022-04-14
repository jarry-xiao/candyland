
import { Program, Provider, } from "@project-serum/anchor";
import {
    Connection,
    Keypair,
    PublicKey
} from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { Gummyroll } from "../target/types/gummyroll";
import { buildTree, getProofOfLeaf, updateTree } from "./merkle-tree";
import { IDL } from '../target/types/gummyroll';

import express from 'express';
const app = express();
const port = 3000;

const PROGRAM_ID = "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD";

const payer = Keypair.generate();
const payerWallet = new NodeWallet(payer);
const connection = new Connection("http://localhost:8899");
const provider = new Provider(connection,
    payerWallet,
    { skipPreflight: true, commitment: "confirmed" }
);
const gummyroll = new Program<Gummyroll>(
    IDL,
    new PublicKey(PROGRAM_ID),
    provider,
);

const eventsProcessed = new Map();
eventsProcessed.set("0", 0);

let BUFFER_SIZE = 1024;
let MAX_DEPTH = 20;

function createEmptyTreeOffChain() {
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    let tree = buildTree(leaves);
    return tree;
}

let tree = createEmptyTreeOffChain();

let listener = gummyroll.addEventListener("ChangeLogEvent", (event) => {
    // console.log(event);
    if (event.index !== undefined) {
        eventsProcessed.set("0", eventsProcessed.get("0") + 1);
        // console.log(event.path, event.path[0].node);
        updateTree(tree, Buffer.from(event.path[0].node.inner), event.index);
    }
});

app.get('/', (req, res) => {
    res.send(`Processed: ${eventsProcessed.get("0")}`)
})

app.get("/proof", (req, res) => {
    const leafIndex = req.query.leafIndex;
    console.log("hit with request for:", leafIndex);

    const proof = getProofOfLeaf(tree, leafIndex).map((node) => Array.from(node.node));
    const result = {
        proof,
        leaf: Array.from(tree.leaves[leafIndex].node),
        root: Array.from(tree.root)
    }
    res.send(JSON.stringify(result));
});

app.listen(port, () => {
    console.log(`Example app listening on port ${port}`)
});
