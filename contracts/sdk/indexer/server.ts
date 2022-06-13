import { PublicKey } from '@solana/web3.js';
import express from 'express';
import { bs58 } from '@project-serum/anchor/dist/cjs/utils/bytes';
import { bootstrap, Proof } from './db';
const app = express();
app.use(express.json());

const port = 3000;

type JsonProof = {
    root: String,
    proofNodes: String[],
    leaf: String,
    index: number
};

function stringifyProof(proof: Proof): string {
    let jsonProof: JsonProof = {
        root: bs58.encode(proof.root),
        proofNodes: proof.proofNodes.map((node) => { return bs58.encode(node) }),
        leaf: bs58.encode(proof.leaf),
        index: proof.index
    }
    return JSON.stringify(jsonProof);
}

app.get("/proof", async (req, res) => {
    const leafHashString = req.query.leafHash;
    console.log("POST request:", leafHashString);
    const nftDb = await bootstrap();
    const leafHash: Buffer = bs58.decode(leafHashString);
    const proof = await nftDb.getProof(leafHash, false);
    res.send(stringifyProof(proof));
});

app.listen(port, () => {
    console.log(`Example app listening on port ${port}`)
});
