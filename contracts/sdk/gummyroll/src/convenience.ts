import { PublicKey, Keypair, TransactionInstruction, SystemProgram, Connection } from "@solana/web3.js";
import { createInitEmptyGummyrollInstruction, PROGRAM_ID } from "./generated";
import * as anchor from "@project-serum/anchor";
import { CANDY_WRAPPER_PROGRAM_ID } from "@sorend-solana/utils";
import { decodeMerkleRoll } from "./accounts";

export function addProof(
  instruction: TransactionInstruction,
  nodeProof: Buffer[],
): TransactionInstruction {
  instruction.keys = instruction.keys.concat(
      nodeProof.map((node) => {
          return {
              pubkey: new PublicKey(node),
              isSigner: false,
              isWritable: false,
          };
      })
  )
  return instruction;
}

export async function getRootOfOnChainMerkleRoot(connection: Connection, merkleRollAccountKey: PublicKey): Promise<Buffer> {
  const merkleRootAcct = await connection.getAccountInfo(merkleRollAccountKey);
  if (!merkleRootAcct) {
      throw new Error("Merkle Root account data unexpectedly null!");
  }
  const merkleRoll = decodeMerkleRoll(merkleRootAcct.data);
  return merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();
}

export function getMerkleRollAccountSize(
  maxDepth: number,
  maxBufferSize: number,
  canopyDepth?: number
): number {
  let headerSize = 8 + 32;
  let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
  let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
  let merkleRollSize = 8 + 8 + 16 + changeLogSize + rightMostPathSize;
  let canopySize = 0;
  if (canopyDepth) {
    canopySize = ((1 << canopyDepth + 1) - 2) * 32
  }
  return merkleRollSize + headerSize + canopySize;
}

export async function createAllocTreeIx(
    connection: Connection,
    maxBufferSize: number,
    maxDepth: number,
    canopyDepth: number,
    payer: PublicKey,
    merkleRoll: PublicKey,
): Promise<TransactionInstruction> {
    const requiredSpace = getMerkleRollAccountSize(
        maxDepth,
        maxBufferSize,
        canopyDepth ?? 0
    );
    return SystemProgram.createAccount({
        fromPubkey: payer,
        newAccountPubkey: merkleRoll,
        lamports:
            await connection.getMinimumBalanceForRentExemption(
                requiredSpace
            ),
        space: requiredSpace,
        programId: PROGRAM_ID
    });
}

