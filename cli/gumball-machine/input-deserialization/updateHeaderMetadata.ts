import { BN } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
    UpdateHeaderMetadataInstructionArgs,
    EncodeMethod
} from "../../../contracts/sdk/gumball-machine";
import {
  assertInRangeAndReturnNum,
  assertLengthAndConvertByteArray,
  assertNonNegativeAndConvertToBN,
  assertLengthAndConvertToPublicKey,
  deserializeCreatorKeys,
  deserializeCreatorShares
} from "./utils";

export function deserializeUpdateHeaderMetadataJson(input): UpdateHeaderMetadataInstructionArgs {
  const gumballMachineUpdateHeaderMetadataArgs: UpdateHeaderMetadataInstructionArgs = {
    urlBase: assertLengthAndConvertByteArray(input.args.urlBase, 64, "urlBase"),
    nameBase: assertLengthAndConvertByteArray(input.args.nameBase, 32, "nameBase"),
    symbol: assertLengthAndConvertByteArray(input.args.symbol, 8, "symbol"),
    sellerFeeBasisPoints: assertInRangeAndReturnNum(input.args.sellerFeeBasisPoints,"sellerFeeBasisPoints", 0, 10000),
    isMutable: input.args.isMutable,
    retainAuthority: input.args.retainAuthority,
    encodeMethod: input.args.encodeMethod,
    price: assertNonNegativeAndConvertToBN(input.args.price, "price"),
    goLiveDate: assertNonNegativeAndConvertToBN(input.args.goLiveDate, "goLiveDate"),
    botWallet: assertLengthAndConvertToPublicKey(input.args.botWallet, "botWallet"),
    authority: assertLengthAndConvertToPublicKey(input.args.authority, "authority"),
    receiver: assertLengthAndConvertToPublicKey(input.args.receiver, "receiver"),
    maxMintSize: assertInRangeAndReturnNum(input.args.maxMintSize, "maxMintSize"),
    creatorKeys: deserializeCreatorKeys(input.args.creatorKeys, input.args.creatorShares),
    creatorShares: deserializeCreatorShares(input.args.creatorShares)
  }
  return gumballMachineUpdateHeaderMetadataArgs;
}