import {
  DispenseNftTokenInstructionArgs
} from "@sorend-solana/gumball-machine";
import {
  assertInRangeAndReturnNum
} from "./utils";

export function deserializeDispenseNFTTokensJson(input): DispenseNftTokenInstructionArgs {
  return {
    numItems: assertInRangeAndReturnNum(input.args.numItems, "numItems")
  }
}