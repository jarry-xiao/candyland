import { BN } from "@project-serum/anchor";
import {
  DispenseNftTokenInstructionArgs
} from "../../../contracts/sdk/gumball-machine";
import {
  assertNonNegativeAndConvertToBN
} from "./utils";

export function deserializeDispenseNFTTokensJson(input): DispenseNftTokenInstructionArgs {
  return {
    numItems: assertNonNegativeAndConvertToBN(input.args.numItems, "numItems")
  }
}