import { ItemPayload } from "./ItemTypes";
import allItems from "./__mocks__/allItems.json";

const MOCK_DATA_BY_OWNER: Record<string, ReadonlyArray<ItemPayload>> = {
  C2jDL4pcwpE2pP5EryTGn842JJUJTcurPGZUquQjySxK: allItems,
};

export default async function getItemsForOwner(
  ownerPubkey: string
): Promise<ReadonlyArray<ItemPayload> | undefined> {
  return MOCK_DATA_BY_OWNER[ownerPubkey];
}
