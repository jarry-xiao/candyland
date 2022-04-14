import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";
import getClient from "../db/getClient";

const MOCK_DATA_BY_OWNER: Record<string, ReadonlyArray<AssetPayload>> = {
  C2jDL4pcwpE2pP5EryTGn842JJUJTcurPGZUquQjySxK: allAssets,
};

export default async function getAssetsForOwner(
  ownerPubkey: string
): Promise<ReadonlyArray<AssetPayload> | undefined> {
  if (!process.env.PGSQL_HOST) {
    return MOCK_DATA_BY_OWNER[ownerPubkey];
  }
  // const client = await getClient();
  // const results = await client?.query("SELECT * from cl_items;");
}
