/**
 * This code was GENERATED using the solita package.
 * Please DO NOT EDIT THIS FILE, instead rerun solita to update it or write a wrapper to add functionality.
 *
 * See: https://github.com/metaplex-foundation/solita
 */

import * as beet from '@metaplex-foundation/beet'
import * as web3 from '@solana/web3.js'

/**
 * @category Instructions
 * @category DispenseNftSol
 * @category generated
 */
export type DispenseNftSolInstructionArgs = {
  numItems: number
}
/**
 * @category Instructions
 * @category DispenseNftSol
 * @category generated
 */
export const dispenseNftSolStruct = new beet.BeetArgsStruct<
  DispenseNftSolInstructionArgs & {
    instructionDiscriminator: number[] /* size: 8 */
  }
>(
  [
    ['instructionDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)],
    ['numItems', beet.u32],
  ],
  'DispenseNftSolInstructionArgs'
)
/**
 * Accounts required by the _dispenseNftSol_ instruction
 *
 * @property [_writable_] gumballMachine
 * @property [_writable_, **signer**] payer
 * @property [_writable_] receiver
 * @property [] willyWonka
 * @property [] recentBlockhashes
 * @property [] instructionSysvarAccount
 * @property [_writable_] bubblegumAuthority
 * @property [_writable_] bubblegumMintRequest
 * @property [] candyWrapper
 * @property [] gummyroll
 * @property [_writable_] merkleSlab
 * @property [] bubblegum
 * @category Instructions
 * @category DispenseNftSol
 * @category generated
 */
export type DispenseNftSolInstructionAccounts = {
  gumballMachine: web3.PublicKey
  payer: web3.PublicKey
  receiver: web3.PublicKey
  willyWonka: web3.PublicKey
  recentBlockhashes: web3.PublicKey
  instructionSysvarAccount: web3.PublicKey
  bubblegumAuthority: web3.PublicKey
  bubblegumMintRequest: web3.PublicKey
  candyWrapper: web3.PublicKey
  gummyroll: web3.PublicKey
  merkleSlab: web3.PublicKey
  bubblegum: web3.PublicKey
}

export const dispenseNftSolInstructionDiscriminator = [
  156, 55, 115, 151, 225, 40, 172, 61,
]

/**
 * Creates a _DispenseNftSol_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category DispenseNftSol
 * @category generated
 */
export function createDispenseNftSolInstruction(
  accounts: DispenseNftSolInstructionAccounts,
  args: DispenseNftSolInstructionArgs
) {
  const {
    gumballMachine,
    payer,
    receiver,
    willyWonka,
    recentBlockhashes,
    instructionSysvarAccount,
    bubblegumAuthority,
    bubblegumMintRequest,
    candyWrapper,
    gummyroll,
    merkleSlab,
    bubblegum,
  } = accounts

  const [data] = dispenseNftSolStruct.serialize({
    instructionDiscriminator: dispenseNftSolInstructionDiscriminator,
    ...args,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: gumballMachine,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: payer,
      isWritable: true,
      isSigner: true,
    },
    {
      pubkey: receiver,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: web3.SystemProgram.programId,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: willyWonka,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: recentBlockhashes,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: instructionSysvarAccount,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: bubblegumAuthority,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: bubblegumMintRequest,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: candyWrapper,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: gummyroll,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: merkleSlab,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: bubblegum,
      isWritable: false,
      isSigner: false,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId: new web3.PublicKey(
      'GBALLoMcmimUutWvtNdFFGH5oguS7ghUUV6toQPppuTW'
    ),
    keys,
    data,
  })
  return ix
}
