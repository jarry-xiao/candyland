import { AssetProof } from "./AssetTypes";
import * as anchor from "@project-serum/anchor";
import getClient from "../db/getClient";

export default async function getProofForAsset(
  treeAccount: anchor.web3.PublicKey,
  index: number
): Promise<AssetProof> {
  if (!process.env.PGSQL_HOST) {
    console.warn(
      "Returning mock proof data. Do not expect any transfer/remove instruction to succeed"
    );
    return {
      hash: Array.from(anchor.web3.PublicKey.default.toBytes()),
      proof: Array.from({ length: 32 }, () =>
        Array.from(anchor.web3.PublicKey.default.toBytes())
      ),
      root: Array.from(anchor.web3.PublicKey.default.toBytes()),
    };
  }
  // const client = await getClient();
  // const results = await client?.query("SELECT * from cl_items;");
  return {
    hash: Array.from(anchor.web3.PublicKey.default.toBytes()),
    proof: Array.from({ length: 32 }, () =>
      Array.from(anchor.web3.PublicKey.default.toBytes())
    ),
    root: Array.from(anchor.web3.PublicKey.default.toBytes()),
  };
}
