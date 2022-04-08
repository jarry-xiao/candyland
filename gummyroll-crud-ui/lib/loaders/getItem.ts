import { ItemPayload } from "./ItemTypes";
import allItems from "./__mocks__/allItems.json";

export default async function getItem(
  treeAccount: string,
  index: number
): Promise<ItemPayload | undefined> {
  return (allItems as ItemPayload[]).find(
    (item) => item.index === index && item.treeAccount === treeAccount
  );
}
