import GummyrollIdl from "../../../target/idl/gummyroll.json";
import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";
import * as anchor from "@project-serum/anchor";
import getProofForAsset from "../loaders/getProofForAsset";

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
    .remove(root, hash, index)
    .accounts({
      authority: treeAdmin,
      authorityPda,
      merkleRoll: treeAccount,
      gummyrollProgram: new anchor.web3.PublicKey(
        GummyrollIdl.metadata.address
      ),
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
