import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";

const MOCK_DATA_BY_OWNER: Record<string, ReadonlyArray<AssetPayload>> = {
  C2jDL4pcwpE2pP5EryTGn842JJUJTcurPGZUquQjySxK: allAssets,
};

export default async function getAssetsForOwner(
  ownerPubkey: string
): Promise<ReadonlyArray<AssetPayload> | undefined> {
  return MOCK_DATA_BY_OWNER[ownerPubkey];
}
