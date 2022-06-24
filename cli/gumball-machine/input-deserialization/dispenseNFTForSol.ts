import { BN } from "@project-serum/anchor";
import {
    DispenseNftSolInstructionArgs
} from "../../../contracts/sdk/gumball-machine";
import {
  assertNonNegativeAndConvertToBN
} from "./utils";

export function deserializeDispenseNFTSolJson(input): DispenseNftSolInstructionArgs {
  return {
    numItems: assertNonNegativeAndConvertToBN(input.args.numItems, "numItems")
  }
}