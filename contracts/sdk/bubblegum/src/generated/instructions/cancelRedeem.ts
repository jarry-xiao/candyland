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
 * @category CancelRedeem
 * @category generated
 */
export type CancelRedeemInstructionArgs = {
  root: number[] /* size: 32 */
}
/**
 * @category Instructions
 * @category CancelRedeem
 * @category generated
 */
export const cancelRedeemStruct = new beet.BeetArgsStruct<
  CancelRedeemInstructionArgs & {
    instructionDiscriminator: number[] /* size: 8 */
  }
>(
  [
    ['instructionDiscriminator', beet.uniformFixedSizeArray(beet.u8, 8)],
    ['root', beet.uniformFixedSizeArray(beet.u8, 32)],
  ],
  'CancelRedeemInstructionArgs'
)
/**
 * Accounts required by the _cancelRedeem_ instruction
 *
 * @property [] authority
 * @property [] candyWrapper
 * @property [] gummyrollProgram
 * @property [_writable_] merkleSlab
 * @property [_writable_] voucher
 * @property [_writable_, **signer**] owner
 * @category Instructions
 * @category CancelRedeem
 * @category generated
 */
export type CancelRedeemInstructionAccounts = {
  authority: web3.PublicKey
  candyWrapper: web3.PublicKey
  gummyrollProgram: web3.PublicKey
  merkleSlab: web3.PublicKey
  voucher: web3.PublicKey
  owner: web3.PublicKey
}

export const cancelRedeemInstructionDiscriminator = [
  111, 76, 232, 50, 39, 175, 48, 242,
]

/**
 * Creates a _CancelRedeem_ instruction.
 *
 * @param accounts that will be accessed while the instruction is processed
 * @param args to provide as instruction data to the program
 *
 * @category Instructions
 * @category CancelRedeem
 * @category generated
 */
export function createCancelRedeemInstruction(
  accounts: CancelRedeemInstructionAccounts,
  args: CancelRedeemInstructionArgs
) {
  const {
    authority,
    candyWrapper,
    gummyrollProgram,
    merkleSlab,
    voucher,
    owner,
  } = accounts

  const [data] = cancelRedeemStruct.serialize({
    instructionDiscriminator: cancelRedeemInstructionDiscriminator,
    ...args,
  })
  const keys: web3.AccountMeta[] = [
    {
      pubkey: authority,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: candyWrapper,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: gummyrollProgram,
      isWritable: false,
      isSigner: false,
    },
    {
      pubkey: merkleSlab,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: voucher,
      isWritable: true,
      isSigner: false,
    },
    {
      pubkey: owner,
      isWritable: true,
      isSigner: true,
    },
  ]

  const ix = new web3.TransactionInstruction({
    programId: new web3.PublicKey(
      'BGUMAp9Gq7iTEuizy4pqaxsTyUCBK68MDfK752saRPUY'
    ),
    keys,
    data,
  })
  return ix
}
