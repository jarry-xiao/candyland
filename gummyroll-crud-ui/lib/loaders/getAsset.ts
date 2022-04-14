import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";
import TreeServerNotConfiguredError from "./TreeServerNotConfiguredError";
import getTreeServerAPIMethod from "./getTreeServerAPIMethod";

export default async function getAsset(
  treeAccount: string,
  index: number
): Promise<AssetPayload | undefined> {
  try {
    const node_index = (1 << 14) + index;
    const asset = await getTreeServerAPIMethod<AssetPayload>(
      `/assets/${treeAccount}/${node_index}`
    );
    console.debug(`API /assets/${treeAccount}/${node_index}`, asset);
    return asset;
  } catch (e) {
    if (e instanceof TreeServerNotConfiguredError) {
      return (allAssets as AssetPayload[]).find(
        (asset) => asset.index === index && asset.treeAccount === treeAccount
      );
    }
    throw e;
  }
}
