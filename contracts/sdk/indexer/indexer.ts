import { Keypair, PublicKey } from "@solana/web3.js";
import { Connection, Context, Logs } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from '../bubblegum/src/generated';
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../gummyroll/index'
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from '../../target/types/gummyroll';
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { readFileSync } from 'fs';
import { loadProgram, parseLogs } from './indexer/utils';
import { parseBubblegum } from "./indexer/bubblegum";

let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function handleLogs(logs: Logs, _context: Context) {
    if (logs.err) {
        return
    }
    // console.log("Sig:", logs.signature);
    const parsed = parseLogs(logs.logs);
    for (const parsedLog of parsed) {
        if (typeof parsedLog !== "string" && parsedLog.programId.equals(BUBBLEGUM_PROGRAM_ID)) {
            parseBubblegum(parsedLog, { Bubblegum, Gummyroll });
        }
    }
}

async function main() {
    const connection = new Connection("http://localhost:8899", "confirmed");

    const keypair = Keypair.generate();
    const provider = new anchor.Provider(connection, new NodeWallet(keypair), { commitment: "confirmed" });

    Gummyroll = loadProgram(provider, GUMMYROLL_PROGRAM_ID, '../../target/idl/gummyroll.json') as anchor.Program<Gummyroll>;
    Bubblegum = loadProgram(provider, BUBBLEGUM_PROGRAM_ID, '../../target/idl/bubblegum.json') as anchor.Program<Bubblegum>;
    console.log("loaded programs...")
    let subscriptionId = connection.onLogs(BUBBLEGUM_PROGRAM_ID, handleLogs);
}
main();
