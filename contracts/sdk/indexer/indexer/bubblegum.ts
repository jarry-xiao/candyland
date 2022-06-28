import {
  ParserState,
  OptionalInfo,
  decodeEvent,
} from "./utils";
import { ParsedLog, dataRegEx } from "./log/bubblegum";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../../gummyroll";
import { ChangeLogEvent, parseEventGummyroll } from "./gummyroll";
import {
  TokenProgramVersion,
  MetadataArgs,
} from "../../bubblegum/src/generated/types";
import { BN, Event, } from "@project-serum/anchor";
import { NFTDatabaseConnection } from "../db";
import { PublicKey } from "@solana/web3.js";


function skipTx(sequenceNumber, startSeq, endSeq): boolean {
  let left = startSeq !== null ? sequenceNumber <= startSeq : false;
  let right = endSeq !== null ? sequenceNumber >= endSeq : false;
  return left || right;
}

export type BubblegumIx =
  | "Redeem"
  | "DecompressV1"
  | "Transfer"
  | "CreateTree"
  | "MintV1"
  | "Burn"
  | "CancelRedeem"
  | "Delegate";

export type NewLeafEvent = {
  version: TokenProgramVersion;
  metadata: MetadataArgs;
  nonce: BN;
};

export type LeafSchemaEvent = {
  schema: {
    v1: {
      id: PublicKey;
      owner: PublicKey;
      delegate: PublicKey;
      nonce: BN;
      dataHash: number[] /* size: 32 */;
      creatorHash: number[] /* size: 32 */;
    };
  };
};

function findGummyrollEvent(
  logs: (string | ParsedLog)[],
  parser: ParserState
): ChangeLogEvent | null {
  let changeLog: ChangeLogEvent | null;
  for (const log of logs) {
    if (typeof log !== "string" && log.programId.equals(GUMMYROLL_PROGRAM_ID)) {
      changeLog = parseEventGummyroll(log, parser.Gummyroll);
    }
  }
  if (!changeLog) {
    console.log("Failed to find gummyroll changelog");
  }
  return changeLog;
}

function findBubblegumEvents(
  logs: (string | ParsedLog)[],
  parser: ParserState
): Array<Event> {
  let events = [];
  for (const log of logs) {
    if (typeof log !== "string") {
      continue;
    }
    let data = log.match(dataRegEx);
    if (data && data.length > 1) {
      events.push(decodeEvent(data[1], parser.Bubblegum.idl));
    }
  }
  return events;
}

export async function ingestBubblegumMint(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const events = findBubblegumEvents(logs, parser);
  if (events.length !== 2) {
    return;
  }
  const newLeafData = events[0].data as NewLeafEvent;
  const leafSchema = events[1].data as LeafSchemaEvent;
  let treeId = changeLog.id.toBase58();
  let sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  await db.updateNFTMetadata(newLeafData, leafSchema.schema.v1.id.toBase58());
  await db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    txId,
    slot,
    sequenceNumber,
    treeId
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function ingestReplaceLeaf(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo,
  compressed: boolean = true
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const events = findBubblegumEvents(logs, parser);
  if (events.length !== 1) {
    return;
  }
  const leafSchema = events[0].data as LeafSchemaEvent;
  let treeId = changeLog.id.toBase58();
  let sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  await db.updateLeafSchema(
    leafSchema,
    new PublicKey(changeLog.path[0].node),
    txId,
    slot,
    sequenceNumber,
    treeId,
    compressed
  );
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function ingestBubblegumCreateTree(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  slot: number,
  parser: ParserState,
  optionalInfo: OptionalInfo
) {
  const changeLog = findGummyrollEvent(logs, parser);
  const sequenceNumber = changeLog.seq;
  let { startSeq, endSeq, txId } = optionalInfo;
  if (skipTx(sequenceNumber, startSeq, endSeq)) {
    return;
  }
  console.log(`Sequence Number: ${sequenceNumber}`);
  let treeId = changeLog.id.toBase58();
  await db.updateChangeLogs(changeLog, optionalInfo.txId, slot, treeId);
}

export async function ingestBubblegumDecompress(
  db: NFTDatabaseConnection,
  logs: (string | ParsedLog)[],
  parser: ParserState,
  optionalInfo: OptionalInfo
) { }
