import { Keypair, PublicKey } from "@solana/web3.js";
import { Connection, Context, Logs } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from '../bubblegum/src/generated';
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../gummyroll/index'
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from '../../target/types/gummyroll';
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { readFileSync } from 'fs';

let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function handleLogs(logs: Logs, _context: Context) {
    if (logs.err) {
        return
    }
    console.log("Sig:", logs.signature);
    const parsed = parseLogs(logs.logs);
    for (const parsedLog of parsed) {
        if (parsedLog.programId.equals(BUBBLEGUM_PROGRAM_ID)) {
            console.log(parsedLog);
        }
    }
}

function handleBubblegumEvent() {

}

function handleGummyrollEvent() {

}

function handleBubbblegumGummyrollEvent() {

}

const startRegEx = /Program (\w*) invoke \[(\d)\]/;
const endRegEx = /Program (\w*) success/;
const dataRegEx = /Program data: ((?:[A-Za-z\d+/]{4})*(?:[A-Za-z\d+/]{3}=|[A-Za-z\d+/]{2}==)?$)/;

type ParsedLog = {
    programId: PublicKey,
    logs: (string | ParsedLog)[]
    depth: number,
}

/**
 * Recursively parses the logs of a program instruction execution
 * @param programId 
 * @param depth 
 * @param logs 
 * @returns 
 */
function parseInstructionLog(programId: PublicKey, depth: number, logs: string[]) {
    const parsedLog: ParsedLog = {
        programId,
        depth,
        logs: [],
    }
    let instructionComplete = false;
    while (!instructionComplete) {
        const logLine = logs[0];
        logs = logs.slice(1);
        let result = logLine.match(endRegEx)
        if (result) {
            if (result[1] != programId.toString()) {
                throw Error(`Unexpected program id finished: ${result[1]}`)
            }
            instructionComplete = true;
        } else {
            result = logLine.match(startRegEx)
            if (result) {
                const programId = new PublicKey(result[1]);
                const depth = Number(result[2]) - 1;
                const parsedInfo = parseInstructionLog(programId, depth, logs);
                parsedLog.logs.push(parsedInfo.parsedLog);
                logs = parsedInfo.logs;
            } else {
                parsedLog.logs.push(logLine);
            }
        }
    }
    return { parsedLog, logs };
}

/**
 * Parses logs so that emitted event data can be tied to its execution context 
 * @param logs 
 * @returns 
 */
function parseLogs(logs: string[]): ParsedLog[] {
    let parsedLogs: ParsedLog[] = [];
    while (logs && logs.length) {
        const logLine = logs[0];
        logs = logs.slice(1);
        const result = logLine.match(startRegEx);
        const programId = new PublicKey(result[1]);
        const depth = Number(result[2]) - 1;
        const parsedInfo = parseInstructionLog(programId, depth, logs)
        parsedLogs.push(parsedInfo.parsedLog);
        logs = parsedInfo.logs;
    }
    return parsedLogs;
}

/**
 * Example: 
 * ```
 * let event = decodeEvent(dataString, Gummyroll.idl) ?? decodeEvent(dataString, Bubblegum.idl);
 * ```
 * @param data 
 * @param idl 
 * @returns 
 */
function decodeEvent(data: string, idl: anchor.Idl): Object | null {
    let eventCoder = new anchor.BorshEventCoder(idl);
    return eventCoder.decode(data);
}

function loadProgram(provider: anchor.Provider, programId: PublicKey, idlPath: string) {
    const IDL = JSON.parse(readFileSync(idlPath).toString());
    return new anchor.Program(IDL, programId, provider)
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
