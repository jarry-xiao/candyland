import { BN, Program } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Connection as web3Connection,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import { GumballMachine } from "../types";
import { getBubblegumAuthorityPDA } from "../../bubblegum/src/convenience";
import {
  InitializeGumballMachineInstructionArgs,
  createInitializeGumballMachineInstruction,
  createDispenseNftSolInstruction,
  createDispenseNftTokenInstruction,
  DispenseNftSolInstructionArgs,
  DispenseNftTokenInstructionArgs
} from "../src/generated";
import { getWillyWonkaPDAKey } from "../utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";

/**
 * Wrapper on top of Solita's createInitializeGumballMachineInstruction
 * Produces a series of instructions to create the merkle roll + gumball machine accounts and initialize gumball machine
 * */
export async function createInitializeGumballMachineIxs(
  payer: Keypair,
  gumballMachineAcctKeypair: Keypair,
  gumballMachineAcctSize: number,
  merkleRollKeypair: Keypair,
  merkleRollAccountSize: number,
  gumballMachineInitArgs: InitializeGumballMachineInstructionArgs,
  mint: PublicKey,
  creator: PublicKey,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>
): Promise<TransactionInstruction[]> {
  const allocGumballMachineAcctInstr = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: gumballMachineAcctKeypair.publicKey,
    lamports:
      await gumballMachine.provider.connection.getMinimumBalanceForRentExemption(
        gumballMachineAcctSize
      ),
    space: gumballMachineAcctSize,
    programId: gumballMachine.programId,
  });

  const allocMerkleRollAcctInstr = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: merkleRollKeypair.publicKey,
    lamports:
      await gumballMachine.provider.connection.getMinimumBalanceForRentExemption(
        merkleRollAccountSize
      ),
    space: merkleRollAccountSize,
    programId: gummyrollProgramId,
  });

  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachineAcctKeypair.publicKey,
    gumballMachine.programId
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollKeypair.publicKey,
  );

  const initGumballMachineInstr = createInitializeGumballMachineInstruction(
    {
      gumballMachine: gumballMachineAcctKeypair.publicKey,
      creator,
      mint,
      willyWonka: willyWonkaPDAKey,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollKeypair.publicKey,
      bubblegum: bubblegumProgramId,
    },
    gumballMachineInitArgs
  );
  return [
    allocGumballMachineAcctInstr,
    allocMerkleRollAcctInstr,
    initGumballMachineInstr,
  ];
}

/**
 * Wrapper on top of Solita's createDispenseNftSolInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForSolIx(
  args: DispenseNftSolInstructionArgs,
  payer: PublicKey,
  numNFTs: BN,
  receiver: PublicKey,
  gumballMachinePubkey: PublicKey,
  merkleRollPubkey: PublicKey,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>,
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(gumballMachinePubkey, gumballMachine.programId);
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollPubkey,
  );
  const dispenseInstr = createDispenseNftSolInstruction(
    {
      gumballMachine: gumballMachinePubkey,
      payer,
      receiver: receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollPubkey,
      bubblegum: bubblegumProgramId,
    },
    args
  );
  return dispenseInstr;
}

/**
 * Wrapper on top of Solita's createDispenseNftTokenInstruction. Automatically fetches necessary PDA keys for instruction
 * */
export async function createDispenseNFTForTokensIx(
  args: DispenseNftTokenInstructionArgs,
  payer: PublicKey,
  numNFTs: BN,
  payerTokens: PublicKey,
  receiver: PublicKey,
  gumballMachinePubkey: PublicKey,
  merkleRollPubkey: PublicKey,
  gummyrollProgramId: PublicKey,
  bubblegumProgramId: PublicKey,
  gumballMachine: Program<GumballMachine>,
): Promise<TransactionInstruction> {
  const willyWonkaPDAKey = await getWillyWonkaPDAKey(
    gumballMachinePubkey,
    gumballMachine.programId
  );
  const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDA(
    merkleRollPubkey,
  );
  const dispenseInstr = createDispenseNftTokenInstruction(
    {
      gumballMachine: gumballMachinePubkey,
      payer,
      payerTokens,
      receiver,
      willyWonka: willyWonkaPDAKey,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
      bubblegumAuthority: bubblegumAuthorityPDAKey,
      candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
      gummyroll: gummyrollProgramId,
      merkleSlab: merkleRollPubkey,
      bubblegum: bubblegumProgramId,
    },
    args
  );
  return dispenseInstr;
}
