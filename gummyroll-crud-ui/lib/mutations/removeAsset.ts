import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";
import * as anchor from "@project-serum/anchor";
import getProofForAsset from "../loaders/getProofForAsset";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

export default async function removeAsset(
  treeAccount: anchor.web3.PublicKey,
  treeAdmin: anchor.web3.PublicKey,
  nodeIndex: number,
  leafIndex: number,
) {
  const program = getGummyrollCrudProgram();
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );

  const { hash, proof, root } = await getProofForAsset(treeAccount, nodeIndex);
  let rootPk = new anchor.web3.PublicKey(root);
  let hashPk = new anchor.web3.PublicKey(hash);

  await program.methods
    .remove(
      Array.from(rootPk.toBytes()),
      Array.from(hashPk.toBytes()),
      leafIndex,
    )
    .accounts({
      authority: treeAdmin,
      authorityPda,
      merkleRoll: treeAccount,
      gummyrollProgram: GummyrollProgramId,
    })
    .remainingAccounts(
      proof.map((pathPart) => ({
        pubkey: new anchor.web3.PublicKey(pathPart),
        isSigner: false,
        isWritable: false,
      }))
    )
    .rpc({ commitment: "confirmed" });
}
