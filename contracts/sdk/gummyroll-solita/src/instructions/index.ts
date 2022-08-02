import { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';
import { 
  InitGummyrollWithRootInstructionArgs,
  InitGummyrollWithRootInstructionAccounts,
  createInitGummyrollWithRootInstruction,
  VerifyLeafInstructionAccounts,
  VerifyLeafInstructionArgs,
  createVerifyLeafInstruction,
  ReplaceLeafInstructionAccounts,
  ReplaceLeafInstructionArgs,
  createReplaceLeafInstruction
} from "../generated"
import {
  addProof
} from "../convenience"

export function createInitGummyrollWithRootWithProofInstruction(
  accts: InitGummyrollWithRootInstructionAccounts,
  args: InitGummyrollWithRootInstructionArgs,
  proof: Buffer[]
): TransactionInstruction {
  let ix = createInitGummyrollWithRootInstruction(accts, args);
  ix = addProof(ix, proof);
  return ix
}

export function createVerifyLeafWithProofInstruction(
  accts: VerifyLeafInstructionAccounts,
  args: VerifyLeafInstructionArgs,
  proof: Buffer[]
): TransactionInstruction {
  let ix = createVerifyLeafInstruction(accts, args);
  ix = addProof(ix, proof);
  return ix
}

export function createReplaceLeafWithProofInstruction(
  accts: ReplaceLeafInstructionAccounts,
  args: ReplaceLeafInstructionArgs,
  proof: Buffer[]
): TransactionInstruction {
  let ix = createReplaceLeafInstruction(accts, args);
  ix = addProof(ix, proof);
  return ix
}