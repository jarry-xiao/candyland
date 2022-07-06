import { BN } from "@project-serum/anchor";
import {
    DispenseNftSolInstructionArgs
} from "../../../contracts/sdk/gumball-machine";
import {
  assertInRangeAndReturnNum
} from "./utils";

export function deserializeDispenseNFTSolJson(input): DispenseNftSolInstructionArgs {
  return {
    numItems: assertInRangeAndReturnNum(input.args.numItems, "numItems")
  }
}