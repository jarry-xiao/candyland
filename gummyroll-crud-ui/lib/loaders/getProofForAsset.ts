import { AssetProof } from "./AssetTypes";
import * as anchor from "@project-serum/anchor";

export default async function getProofForAsset(
  treeAccount: anchor.web3.PublicKey,
  index: number
): Promise<AssetProof> {
  if (!process.env.NEXT_PUBLIC_TREE_SERVER_API_ENDPOINT) {
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
  const response = await fetch(
    new URL(
      `/asset/${treeAccount.toBase58()}/${index}/proof`,
      process.env.NEXT_PUBLIC_TREE_SERVER_API_ENDPOINT
    ).toString()
  );
  return await response.json();
}
