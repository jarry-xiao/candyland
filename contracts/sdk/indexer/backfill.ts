import { Keypair, PublicKey } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../bubblegum/src/generated";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../gummyroll/index";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadProgram, handleLogs, handleLogsAtomic } from "./indexer/utils";
import { bootstrap, NFTDatabaseConnection } from "./db";
import { backfillTreeHistory, fillGapsTx, validateTree } from "./backfiller";

const url = "http://api.internal.mainnet-beta.solana.com";
// const url = "http://127.0.0.1:8899";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function main() {
    const treeId = process.argv[2];
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = Keypair.generate();
    const provider = new anchor.Provider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });
    let db = await bootstrap();
    console.log("Finished bootstrapping DB");
    Gummyroll = loadProgram(
        provider,
        GUMMYROLL_PROGRAM_ID,
        "target/idl/gummyroll.json"
    ) as anchor.Program<Gummyroll>;
    Bubblegum = loadProgram(
        provider,
        BUBBLEGUM_PROGRAM_ID,
        "target/idl/bubblegum.json"
    ) as anchor.Program<Bubblegum>;
    console.log("loaded programs...");
    console.log("Filling in gaps for tree:", treeId);

    const parserState = { Gummyroll, Bubblegum };

    // Fill gaps
    let { maxSeq, maxSeqSlot } = await fillGapsTx(connection, db, parserState, treeId);

    // Backfill to on-chain state, now with a complete db
    console.log(`Starting from slot!: ${maxSeqSlot} `);
    maxSeq = await backfillTreeHistory(connection, db, parserState, treeId, maxSeq, maxSeqSlot);

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
