import {
  assertInRangeAndReturnNum
} from "./utils";

export function deserializeInitIndicesJson(input) {
  const maxItems = assertInRangeAndReturnNum(input.args.maxItems, "maxItems");
  return {
    maxItems
  }
}