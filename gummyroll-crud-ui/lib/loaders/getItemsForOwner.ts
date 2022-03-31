import { ItemPayload } from "./ItemTypes";
import allItems from "./__mocks__/allItems.json";

const MOCK_DATA_BY_OWNER: Record<string, ReadonlyArray<ItemPayload>> = {
  aJ69C1ZjyGM2eeZknnkEQ6hjA48dKCIyqfoZaHXZFDz: allItems,
};

export default async function getItemsForOwner(
  ownerPubkey: string
): Promise<ReadonlyArray<ItemPayload> | undefined> {
  return MOCK_DATA_BY_OWNER[ownerPubkey];
}
