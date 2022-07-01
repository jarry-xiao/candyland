import {
    AddConfigLinesInstructionArgs
} from "../../../contracts/sdk/gumball-machine";
import {
  getBufferFromStringArr
} from "./utils";

export function deserializeAddConfigLinesJson(input): AddConfigLinesInstructionArgs {
  const newConfigLinesData = getBufferFromStringArr(input.args.newConfigLines);
  return {
    newConfigLinesData
  }
}