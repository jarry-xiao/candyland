import {
    DispenseNftSolInstructionArgs
} from "@sorend-solana/gumball-machine";
import {
  assertInRangeAndReturnNum
} from "./utils";

export function deserializeDispenseNFTSolJson(input): DispenseNftSolInstructionArgs {
  return {
    numItems: assertInRangeAndReturnNum(input.args.numItems, "numItems")
  }
}