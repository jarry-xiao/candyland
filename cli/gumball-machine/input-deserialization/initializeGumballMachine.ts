import log from 'loglevel';
import { BN, Provider, Program } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
    gumballMachineHeaderBeet,
    InitializeGumballMachineInstructionArgs,
    EncodeMethod
} from "../../../contracts/sdk/gumball-machine";
import { NATIVE_MINT } from "@solana/spl-token";
import {
  getMerkleRollAccountSize,
} from "../../../contracts/sdk/gummyroll";
import {
    val,
    strToByteArray,
    strToByteUint8Array
} from "../../../contracts/sdk/utils/index";
import {
  assertInRangeAndReturnNum,
  assertLengthAndConvertByteArray,
  assertNonNegativeAndConvertToBN,
  assertLengthAndConvertToPublicKey,
} from "./utils";

function deserializeCreatorKeys(keys: string[]): PublicKey[] {
  if (keys.length > 5) {
    throw new Error(`❌ creatorKeys is too long! We currently only support at most 5 creators ❌`);
  } else {
    return keys.map((key, i) => assertLengthAndConvertToPublicKey(key, `Creator key ${i}`))
  }
}

function deserializeCreatorShares(shares: number[]): Uint8Array {
  if (shares.reduce((acc, share) => acc + share, 0) > 100) {
    throw new Error(`❌ creatorShares cannot sum to more than 100% ❌`);
  }
  return Uint8Array.from(shares);
}

export function deserializeInitJson(input): [InitializeGumballMachineInstructionArgs, number, number] {
  const gumballMachineInitArgs: InitializeGumballMachineInstructionArgs = {
    maxDepth: assertInRangeAndReturnNum(input.args.maxDepth, "maxDepth"),
    maxBufferSize: assertInRangeAndReturnNum(input.args.maxBufferSize, "maxBufferSize"),
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
    receiver: assertLengthAndConvertToPublicKey(input.args.receiver, "receiver"),
    authority: assertLengthAndConvertToPublicKey(input.args.authority, "authority"),
    collectionKey: input.args.collectionKey === null ? SystemProgram.programId : assertLengthAndConvertToPublicKey(input.args.collectionKey, "collectionKey"),
    extensionLen: assertNonNegativeAndConvertToBN(input.args.extensionLen, "extensionLen"),
    maxMintSize: assertInRangeAndReturnNum(input.args.maxMintSize, "maxMintSize"),
    maxItems: assertInRangeAndReturnNum(input.args.maxItems, "maxItems"),
    creatorKeys: deserializeCreatorKeys(input.args.creatorKeys),
    creatorShares: deserializeCreatorShares(input.args.creatorShares)
  }

  const GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE = gumballMachineInitArgs.maxItems * 4;
  const GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = val(gumballMachineInitArgs.extensionLen).toNumber() * gumballMachineInitArgs.maxItems;
  const GUMBALL_MACHINE_ACCT_SIZE =
    gumballMachineHeaderBeet.byteSize +
    GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE +
    GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
  
  let MERKLE_ROLL_ACCT_SIZE;
  if ("optionals" in input && "canopyDepth" in input.optionals) {
    MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(gumballMachineInitArgs.maxDepth, gumballMachineInitArgs.maxBufferSize, input.optionals.canopyDepth);
  } else {
    MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(gumballMachineInitArgs.maxDepth, gumballMachineInitArgs.maxBufferSize);
  }
  return [gumballMachineInitArgs, GUMBALL_MACHINE_ACCT_SIZE, MERKLE_ROLL_ACCT_SIZE];
}
