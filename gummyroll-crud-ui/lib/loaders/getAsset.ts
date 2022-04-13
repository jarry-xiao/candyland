import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";

export default async function getasset(
  treeAccount: string,
  index: number
): Promise<AssetPayload | undefined> {
  return (allAssets as AssetPayload[]).find(
    (asset) => asset.index === index && asset.treeAccount === treeAccount
  );
}
