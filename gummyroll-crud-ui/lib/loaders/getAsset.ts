import { AssetPayload } from "./AssetTypes";
import allAssets from "./__mocks__/allAssets.json";
import getClient from "../db/getClient";

export default async function getasset(
  treeAccount: string,
  index: number
): Promise<AssetPayload | undefined> {
  if (!process.env.PGSQL_HOST) {
    return (allAssets as AssetPayload[]).find(
      (asset) => asset.index === index && asset.treeAccount === treeAccount
    );
  }
  // const client = await getClient();
  // const results = await client?.query("SELECT * from cl_items;");
}
