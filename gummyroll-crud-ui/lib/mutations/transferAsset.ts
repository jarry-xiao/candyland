import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";
import * as anchor from "@project-serum/anchor";
import getProofForAsset from "../loaders/getProofForAsset";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

export default async function transferAsset(
  treeAccount: anchor.web3.PublicKey,
  treeAdmin: anchor.web3.PublicKey,
  data: Buffer,
  nodeIndex: number,
  leafIndex: number,
  owner: anchor.web3.PublicKey,
  newOwner: anchor.web3.PublicKey
) {
  const program = getGummyrollCrudProgram();
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );
  const { proof, root } = await getProofForAsset(treeAccount, nodeIndex);
  let rootPk = new anchor.web3.PublicKey(root);

  await program.methods
    .transfer(Array.from(rootPk.toBytes()), data, leafIndex)
    .accounts({
      authority: treeAdmin,
      authorityPda,
      merkleRoll: treeAccount,
      owner,
      newOwner,
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
