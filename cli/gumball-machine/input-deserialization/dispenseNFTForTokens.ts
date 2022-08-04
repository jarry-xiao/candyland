import { BN } from "@project-serum/anchor";
import {
  DispenseNftTokenInstructionArgs
} from "../../../contracts/sdk/gumball-machine";
import {
  assertInRangeAndReturnNum
} from "./utils";

export function deserializeDispenseNFTTokensJson(input): DispenseNftTokenInstructionArgs {
  return {
    numItems: assertInRangeAndReturnNum(input.args.numItems, "numItems")
  }
}