import * as anchor from "@project-serum/anchor";
import { CompiledInnerInstruction, CompiledInstruction, Context, Logs, PublicKey } from "@solana/web3.js";
import { readFileSync } from "fs";
import { Bubblegum } from "../../../target/types/bubblegum";
import { Gummyroll } from "../../../target/types/gummyroll";
import { NFTDatabaseConnection } from "../db";
import { parseBubblegumInstruction } from "./instruction/bubblegum";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../../gummyroll";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../../bubblegum/src/generated";

export type ParserState = {
  Gummyroll: anchor.Program<Gummyroll>;
  Bubblegum: anchor.Program<Bubblegum>;
};


export type OptionalInfo = {
  txId: string;

  startSeq: number | null;
  endSeq: number | null;
};


/**
 * Example:
 * ```
 * let event = decodeEvent(dataString, Gummyroll.idl) ?? decodeEvent(dataString, Bubblegum.idl);
 * ```
 * @param data
 * @param idl
 * @returns
 */
export function decodeEvent(data: string, idl: anchor.Idl): anchor.Event | null {
  let eventCoder = new anchor.BorshEventCoder(idl);
  return eventCoder.decode(data);
}

export function loadProgram(
  provider: anchor.Provider,
  programId: PublicKey,
  idlPath: string
) {
  const IDL = JSON.parse(readFileSync(idlPath).toString());
  return new anchor.Program(IDL, programId, provider);
}

export enum ParseResult {
  Success,
  LogTruncated,
  TransactionError
};

function indexZippedInstruction(
  db: NFTDatabaseConnection,
  context: { txId: string, startSeq: number, endSeq: number },
  slot: number,
  parserState: ParserState,
  accountKeys: PublicKey[],
  zippedInstruction: ZippedInstruction,
) {
  const { instruction, innerInstructions } = zippedInstruction;
  const programId = accountKeys[instruction.programIdIndex];
  if (programId.equals(BUBBLEGUM_PROGRAM_ID)) {
    console.log("Found bubblegum");
    parseBubblegumInstruction(
      db,
      slot,
      parserState,
      context,
      accountKeys,
      instruction,
      innerInstructions
    );
  } else {
    /// TODO: test with gumball-machine truncate mode
    /// TODO: write inner instruction parser
    console.log("[no outer bgum ix found] Ignoring for now");
  }
}

type ZippedInstruction = {
  instructionIndex: number,
  instruction: CompiledInstruction,
  innerInstructions: CompiledInnerInstruction[],
}

/// Similar to `order_instructions` in `/nft_ingester/src/utils/instructions.rs`
function zipInstructions(
  instructions: CompiledInstruction[],
  innerInstructions: CompiledInnerInstruction[],
): ZippedInstruction[] {
  const zippedIxs = [];
  let innerIxIndex = 0;
  for (let instructionIndex = 0; instructionIndex < instructions.length; instructionIndex++) {
    const innerIxs = [];
    while (innerInstructions[innerIxIndex].index < instructionIndex) {
      innerIxs.push(innerInstructions[innerIxIndex]);
      innerIxIndex += 1;
    }
    zippedIxs.push({
      instructionIndex,
      instruction: instructions[instructionIndex],
      innerIxs
    })
  }
  return zippedIxs;
}

export function handleInstructionsAtomic(
  db: NFTDatabaseConnection,
  instructionInfo: {
    accountKeys: PublicKey[],
    instructions: CompiledInstruction[],
    innerInstructions: CompiledInnerInstruction[],
  },
  txId: string,
  context: Context,
  parsedState: ParserState,
  startSeq: number | null = null,
  endSeq: number | null = null
) {
  const { accountKeys, instructions, innerInstructions } = instructionInfo;

  const zippedInstructions = zipInstructions(instructions, innerInstructions);
  for (let i = 0; i < zippedInstructions.length; i++) {
    indexZippedInstruction(
      db,
      { txId, startSeq, endSeq },
      context.slot,
      parsedState,
      accountKeys,
      zippedInstructions[i],
    )
  }
}

export function loadPrograms(provider: anchor.Provider) {
  const Gummyroll = loadProgram(
    provider,
    GUMMYROLL_PROGRAM_ID,
    "target/idl/gummyroll.json"
  ) as anchor.Program<Gummyroll>;
  const Bubblegum = loadProgram(
    provider,
    BUBBLEGUM_PROGRAM_ID,
    "target/idl/bubblegum.json"
  ) as anchor.Program<Bubblegum>;
  return { Gummyroll, Bubblegum };
}
