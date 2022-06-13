import { ParsedLog, ParserState, ixRegEx, parseEventFromLog, OptionalInfo } from "./utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../../gummyroll';
import { ChangeLogEvent, parseEventGummyroll } from "./gummyroll";
import { handleBubblegumCreateTree, handleBubblegumMint } from '../ingester/bubblegum';
import { LeafSchema, TokenProgramVersion, MetadataArgs } from '../../bubblegum/src/generated/types';
import { BN } from "@project-serum/anchor";

function parseIxName(logLine: string): BubblegumIx | null {
    return logLine.match(ixRegEx)[1] as BubblegumIx
}

export type BubblegumIx =
    'Redeem' | 'Decompress' | 'Transfer' | 'CreateTree' | 'Mint' | 'CancelRedeem' | 'Delegate';

export type NewLeafEvent = {
    version: TokenProgramVersion,
    metadata: MetadataArgs,
    nonce: BN,
}

export function parseBubblegum(parsedLog: ParsedLog, parser: ParserState, optionalInfo: OptionalInfo) {
    const ixName = parseIxName(parsedLog.logs[0] as string);
    console.log("Bubblegum:", ixName);
    switch (ixName) {
        case 'CreateTree':
            parseBubblegumCreateTree(parsedLog.logs, parser, optionalInfo);
            break
        case 'Mint':
            parseBubblegumMint(parsedLog.logs, parser, optionalInfo);
            break
        case 'Redeem':
            parseBubblegumRedeem();
            break
        case 'CancelRedeem':
            parseBubblegumCancelRedeem()
            break
        case 'Transfer':
            parseBubblegumTransfer();
            break
        case 'Delegate':
            parseBubblegumDelegate();
            break
    }
}

function findGummyrollEvent(logs: (string | ParsedLog)[], parser: ParserState): ChangeLogEvent | null {
    let changeLog: ChangeLogEvent | null;
    for (const log of logs) {
        if (typeof log !== 'string' && log.programId.equals(GUMMYROLL_PROGRAM_ID)) {
            changeLog = parseEventGummyroll(log, parser.Gummyroll);
        }
    }
    if (!changeLog) {
        console.log("Failed to find gummyroll changelog");
    }
    return changeLog;
}

export function parseBubblegumMint(logs: (string | ParsedLog)[], parser: ParserState, optionalInfo: OptionalInfo) {
    const changeLog = findGummyrollEvent(logs, parser);
    const newLeafData = parseEventFromLog(logs[1] as string, parser.Bubblegum.idl).data as NewLeafEvent;
    const leafSchema = parseEventFromLog(logs[2] as string, parser.Bubblegum.idl).data as LeafSchema;
    handleBubblegumMint(newLeafData, leafSchema, changeLog, optionalInfo);
}

export function parseBubblegumTransfer() {

}

export function parseBubblegumCreateTree(logs: (string | ParsedLog)[], parser: ParserState, optionalInfo: OptionalInfo) {
    const changeLog = findGummyrollEvent(logs, parser);
    handleBubblegumCreateTree(changeLog, optionalInfo);
}

export function parseBubblegumDelegate() {

}

export function parseBubblegumRedeem() {

}

export function parseBubblegumCancelRedeem() {

}

export function parseBubblegumDecompress() {

}
