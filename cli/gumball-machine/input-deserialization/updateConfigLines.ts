import {
    UpdateConfigLinesInstructionArgs
} from "@sorend-solana/gumball-machine";
import {
  getBufferFromStringArr,
  assertNonNegativeAndConvertToBN
} from "./utils";

export function deserializeUpdateConfigLinesJson(input): UpdateConfigLinesInstructionArgs {
  const newConfigLinesData = getBufferFromStringArr(input.args.newConfigLines);
  return {
    startingLine: assertNonNegativeAndConvertToBN(input.args.startingLine, "startingLine"),
    newConfigLinesData
  }
}