import { AssetProof } from "./AssetTypes";
import * as anchor from "@project-serum/anchor";
import getTreeServerAPIMethod from "./getTreeServerAPIMethod";
import TreeServerNotConfiguredError from "./TreeServerNotConfiguredError";

export default async function getProofForAsset(
  treeAccount: anchor.web3.PublicKey,
  index: number
): Promise<AssetProof> {
  try {
    let proof = await getTreeServerAPIMethod<AssetProof>(
      `/assets/${treeAccount.toBase58()}/${index}/proof`
    );
    // Super dirty hack, we will fix for prod
    proof.proof.pop();
    console.log(`API /assets/${treeAccount.toBase58()}/${index}/proof`, proof);
    return proof;
  } catch (e) {
    if (e instanceof TreeServerNotConfiguredError) {
      console.warn(
        "Returning mock proof data. Do not expect any transfer/remove instruction to succeed"
      );
      return {
        hash: anchor.web3.PublicKey.default.toString(),
        proof: Array.from({ length: 32 }, () =>
          anchor.web3.PublicKey.default.toString()
        ),
        root: anchor.web3.PublicKey.default.toString(),
      };
    }
    throw e;
  }
}
