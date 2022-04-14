import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";
import getTreeServerAPIMethod from "./getTreeServerAPIMethod";
import TreeServerNotConfiguredError from "./TreeServerNotConfiguredError";

const MOCK_DATA_BY_OWNER: Record<string, ReadonlyArray<AssetPayload>> = {
  C2jDL4pcwpE2pP5EryTGn842JJUJTcurPGZUquQjySxK: allAssets,
};

export default async function getAssetsForOwner(
  ownerPubkey: string
): Promise<ReadonlyArray<AssetPayload> | undefined> {
  try {
    const assets = await getTreeServerAPIMethod<AssetPayload[]>(
      `/owner/${ownerPubkey}/assets`
    );
    console.debug(`API /owner/${ownerPubkey}/assets`, assets);
    return assets;
  } catch (e) {
    if (e instanceof TreeServerNotConfiguredError) {
      return MOCK_DATA_BY_OWNER[ownerPubkey];
    }
    throw e;
  }
}
