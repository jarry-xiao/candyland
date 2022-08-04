import { Keypair, PublicKey } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadPrograms } from "./indexer/utils";
import { bootstrap } from "./db";
import { backfillTreeHistory, fillGapsTx, validateTree } from "./backfiller";

// const url = "http://api.explorer.mainnet-beta.solana.com";
// const url = "http://127.0.0.1:8899";
const url = "https://api.devnet.solana.com";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function main() {
    const treeId = process.argv[2];
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = Keypair.generate();
    const provider = new anchor.AnchorProvider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });
    let db = await bootstrap();
    console.log("Finished bootstrapping DB");

    const parserState = loadPrograms(provider);
    console.log("loaded programs...");

    let earliestTxIdForTree = null;
    let maxSeq: number, maxSeqSlot: number;
    try {
        const result = await db.connection.all('select min(seq), transaction_id, tree_id from leaf_schema where tree_id = ?;', treeId.toString());
        if (result.length !== 1) {
            throw new Error("No result from SQL")
        }
        earliestTxIdForTree = result[0].transaction_id as string;
        // not sure what to do here


        // Fill gaps
        console.log("Filling in gaps for tree:", treeId);
        let gapResult = await fillGapsTx(connection, db, parserState, treeId);
        maxSeq = gapResult.maxSeq;
        maxSeqSlot = gapResult.maxSeqSlot;

        // Backfill to on-chain state, now with a complete db
        console.log(`Starting from slot!: ${maxSeqSlot} `);
        maxSeq = await backfillTreeHistory(connection, db, parserState, treeId, maxSeq, maxSeqSlot);
    } catch {
        earliestTxIdForTree = null;
        const currentSeq = await connection.getSlot('confirmed');
        console.log("Current seq", currentSeq);
        maxSeq = await backfillTreeHistory(connection, db, parserState, treeId, Number.MAX_SAFE_INTEGER, currentSeq);
    }


    // Validate
    console.log("Max SEQUENCE: ", maxSeq);
    const depth = await db.getDepth(treeId);
    console.log(`Tree ${treeId} has ${depth}`);
    console.log("Validating")
    console.log(
        `    Off - chain tree ${treeId} is consistent: ${await validateTree(
            db,
            depth,
            treeId,
            maxSeq,
        )
        } `
    );
}

main();
