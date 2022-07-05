import {
  ParserState,
  OptionalInfo,
  decodeEvent,
} from "./utils";
import { ParsedLog } from "./log/utils";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID, PathNode } from "../../gummyroll";
import {
  TokenProgramVersion,
  MetadataArgs,
} from "../../bubblegum/src/generated/types";
import { BN, } from "@project-serum/anchor";
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

export type ChangeLogEvent = {
  id: PublicKey,
  path: PathNode[],
  seq: number,
  index: number,
};

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


export async function ingestBubblegumMint(
  db: NFTDatabaseConnection,
  slot: number,
  optionalInfo: OptionalInfo,
  changeLog: ChangeLogEvent,
  newLeafData: NewLeafEvent,
  leafSchema: LeafSchemaEvent,
) {
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

export async function ingestBubblegumReplaceLeaf(
  db: NFTDatabaseConnection,
  slot: number,
  optionalInfo: OptionalInfo,
  changeLog: ChangeLogEvent,
  leafSchema: LeafSchemaEvent,
  compressed: boolean = true
) {
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
  slot: number,
  optionalInfo: OptionalInfo,
  changeLog: ChangeLogEvent
) {
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
