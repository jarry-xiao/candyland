import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";
import * as anchor from "@project-serum/anchor";
import getProofForAsset from "../loaders/getProofForAsset";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

export default async function removeAsset(
  treeAccount: anchor.web3.PublicKey,
  treeAdmin: anchor.web3.PublicKey,
  index: number
) {
  const program = getGummyrollCrudProgram();
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );
  const { hash, proof, root } = await getProofForAsset(treeAccount, index);
  await program.methods
    .remove(
      Buffer.from(root, "utf-8").toJSON().data,
      Buffer.from(hash, "utf-8").toJSON().data,
      index
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
