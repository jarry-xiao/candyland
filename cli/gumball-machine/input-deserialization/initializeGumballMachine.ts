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
} from "@sorend-solana/gumball-machine";
import {
  getMerkleRollAccountSize,
} from "@sorend-solana/gummyroll-solita";
import {
    val,
    strToByteArray,
    strToByteUint8Array
} from "@sorend-solana/utils";
import {
  assertInRangeAndReturnNum,
  assertLengthAndConvertByteArray,
  assertNonNegativeAndConvertToBN,
  assertLengthAndConvertToPublicKey,
  deserializeCreatorKeys,
  deserializeCreatorShares
} from "./utils";


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
    creatorKeys: deserializeCreatorKeys(input.args.creatorKeys, input.args.creatorShares),
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
