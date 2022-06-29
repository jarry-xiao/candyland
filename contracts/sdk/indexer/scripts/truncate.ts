import { Keypair, Connection, TransactionResponse, PublicKey } from "@solana/web3.js";
import * as anchor from '@project-serum/anchor';
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";
import { getBubblegumAuthorityPDA, getCreateTreeIxs, getLeafAssetId } from "../../bubblegum/src/convenience";
import { addProof, PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../../gummyroll';
import {
    TokenStandard,
    MetadataArgs,
    TokenProgramVersion,
    createTransferInstruction,
    createMintV1Instruction,
    LeafSchema,
    leafSchemaBeet,
} from "../../bubblegum/src/generated";
import { execute } from "../../../tests/utils";
import { hashCreators, hashMetadata } from "../indexer/instruction/bubblegum";
import { BN } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import fetch from "node-fetch";
import { keccak_256 } from 'js-sha3';
import { BinaryWriter } from 'borsh';

// const url = "http://api.explorer.mainnet-beta.solana.com";
const url = "http://127.0.0.1:8899";

function keypairFromString(seed: string) {
    const spaces = "                                         ";
    const buffer = Buffer.from(`${seed}${spaces}`.slice(0, 32));;
    return Keypair.fromSeed(Uint8Array.from(buffer));
}

const MAX_BUFFER_SIZE = 256;
const MAX_DEPTH = 20;
const CANOPY_DEPTH = 5;

/**
 * Truncates logs by sending too many append instructions
 * This forces the indexer to go into gap-filling mode
 * and use the WRAP CPI args to complete the database.
 */
async function main() {
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = keypairFromString('bubblegum-mini-milady');
    const provider = new anchor.Provider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });

    // TODO: add gumball-machine version of truncate(test cpi indexing using instruction data)
    let { txId, tx } = await truncateViaBubblegum(connection, provider, payer);
    checkTxTruncated(tx);

    // TOOD: add this after gumball-machine mints
    let results = await testWithBubblegumTransfers(connection, provider, payer);

    results.txs.map((tx) => {
        checkTxTruncated(tx);
    })
}

function checkTxTruncated(tx: TransactionResponse) {
    if (tx.meta.logMessages) {
        let logsTruncated = false;
        for (const log of tx.meta.logMessages) {
            if (log.startsWith('Log truncated')) {
                logsTruncated = true;
            }
        }
        console.log(`Logs truncated: ${logsTruncated}`);
    } else {
        console.error("NO LOG MESSAGES FOUND AT ALL...error!!!")
    }
}

function getMetadata(num: number): MetadataArgs {
    return {
        name: `${num}`,
        symbol: `MILADY`,
        uri: "http://remilia.org",
        sellerFeeBasisPoints: 0,
        primarySaleHappened: false,
        isMutable: false,
        uses: null,
        collection: null,
        creators: [],
        tokenProgramVersion: TokenProgramVersion.Original,
        tokenStandard: TokenStandard.NonFungible,
        editionNonce: 0,
    }
}

async function truncateViaBubblegum(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
) {
    const bgumTree = keypairFromString("bubblegum-mini-tree");
    const authority = await getBubblegumAuthorityPDA(bgumTree.publicKey);

    const acctInfo = await connection.getAccountInfo(bgumTree.publicKey, "confirmed");
    let createIxs = [];
    if (!acctInfo || acctInfo.lamports === 0) {
        console.log("Creating tree:", bgumTree.publicKey.toBase58());
        console.log("Requesting airdrop:", await connection.requestAirdrop(payer.publicKey, 5e10));
        createIxs = await getCreateTreeIxs(connection, MAX_DEPTH, MAX_BUFFER_SIZE, CANOPY_DEPTH, payer.publicKey, bgumTree.publicKey, payer.publicKey);
        console.log("<Creating tree in the truncation tx>");
    } else {
        console.log("Bubblegum tree already exists:", bgumTree.publicKey.toBase58());
    }

    const mintIxs = [];
    for (let i = 0; i < 6; i++) {
        const metadata = getMetadata(i);
        mintIxs.push(createMintV1Instruction(
            {
                owner: payer.publicKey,
                delegate: payer.publicKey,
                authority,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                gummyrollProgram: GUMMYROLL_PROGRAM_ID,
                mintAuthority: payer.publicKey,
                merkleSlab: bgumTree.publicKey,
            },
            { message: metadata }
        ));
    }
    console.log("Sending multiple mint ixs in a transaction");
    const ixs = createIxs.concat(mintIxs);
    const txId = await execute(provider, ixs, [payer, bgumTree], true);
    console.log(`Executed multiple mint ixs here: ${txId}`);
    const tx = await connection.getTransaction(txId, { commitment: 'confirmed' });
    return { txId, tx };
}

type ProofResult = {
    dataHash: number[],
    creatorHash: number[],
    root: number[],
    proofNodes: Buffer[],
    nonce: number,
    index: number,
}

async function getTransferInfoFromServer(leafHash: Buffer, treeId: PublicKey): Promise<ProofResult> {
    const proofServerUrl = "http://127.0.0.1:4000/proof";
    const hash = bs58.encode(leafHash);
    const url = `${proofServerUrl}?leafHash=${hash}&treeId=${treeId.toString()}`;
    const response = await fetch(
        url,
        { method: "GET" }
    );
    const proof = await response.json();
    return {
        dataHash: [...bs58.decode(proof.dataHash as string)],
        creatorHash: [...bs58.decode(proof.creatorHash as string)],
        root: [...bs58.decode(proof.root as string)],
        proofNodes: (proof.proofNodes as string[]).map((node) => bs58.decode(node)),
        nonce: proof.nonce,
        index: proof.index,
    };
}

// todo: expose somewhere in utils
function digest(input: Buffer): Buffer {
    return Buffer.from(keccak_256.digest(input))
}

/// Typescript impl of LeafSchema::to_node()
function hashLeafSchema(leafSchema: LeafSchema, dataHash: Buffer, creatorHash: Buffer): Buffer {
    // Fix issue with solita, the following code should work, but doesn't seem to
    // const result = leafSchemaBeet.toFixedFromValue(leafSchema);
    // const buffer = Buffer.alloc(result.byteSize);
    // result.write(buffer, 0, leafSchema);

    const writer = new BinaryWriter();
    // When we have versions other than V1, we definitely want to use solita
    writer.writeU8(1);
    writer.writeFixedArray(leafSchema.id.toBuffer());
    writer.writeFixedArray(leafSchema.owner.toBuffer());
    writer.writeFixedArray(leafSchema.delegate.toBuffer());
    writer.writeFixedArray(new BN(leafSchema.nonce).toBuffer('le', 8));
    writer.writeFixedArray(dataHash);
    writer.writeFixedArray(creatorHash);
    const buf = Buffer.from(writer.toArray());
    return digest(buf);
}

async function testWithBubblegumTransfers(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
) {
    const bgumTree = keypairFromString("bubblegum-mini-tree");
    const authority = await getBubblegumAuthorityPDA(bgumTree.publicKey);

    // const acctInfo = await connection.getAccountInfo(bgumTree.publicKey, "confirmed");
    // const merkleRoll = decodeMerkleRoll(acctInfo.data);
    // const root = Array.from(merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBytes());

    const txIds = [];
    const txs = [];
    const finalDestination = keypairFromString("bubblegum-final-destination");
    for (let i = 0; i < 6; i++) {
        const metadata = getMetadata(i);
        const computedDataHash = hashMetadata(metadata);
        const computedCreatorHash = hashCreators(metadata.creators);
        const leafSchema: LeafSchema = {
            __kind: "V1",
            id: await getLeafAssetId(bgumTree.publicKey, new BN(i)),
            owner: payer.publicKey,
            delegate: payer.publicKey,
            nonce: new BN(i),
            dataHash: [...computedDataHash],
            creatorHash: [...computedCreatorHash],
        };
        const leafHash = hashLeafSchema(leafSchema, computedDataHash, computedCreatorHash);
        console.log("Data hash:", bs58.encode(computedDataHash));
        console.log("Creator hash:", bs58.encode(computedCreatorHash));
        console.log("schema:", {
            id: leafSchema.id.toString(),
            owner: leafSchema.owner.toString(),
            delegate: leafSchema.owner.toString(),
            nonce: new BN(i),
        });
        const { root, dataHash, creatorHash, proofNodes, nonce, index } = await getTransferInfoFromServer(leafHash, bgumTree.publicKey);
        const transferIx = addProof(createTransferInstruction({
            authority,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
            gummyrollProgram: GUMMYROLL_PROGRAM_ID,
            owner: payer.publicKey,
            delegate: payer.publicKey,
            newOwner: finalDestination.publicKey,
            merkleSlab: bgumTree.publicKey,
        }, {
            dataHash,
            creatorHash,
            nonce,
            root,
            index,
        }), proofNodes.slice(0, MAX_DEPTH - CANOPY_DEPTH));
        txIds.push(await execute(provider, [transferIx], [payer], true));
        txs.push(await connection.getTransaction(txIds[txIds.length - 1], { commitment: 'confirmed' }));
    }
    console.log(`Transferred all NFTs to ${finalDestination.publicKey.toString()}`);
    console.log(`Executed multiple transfer ixs here: ${txIds}`);
    return { txIds, txs };
}

main();
