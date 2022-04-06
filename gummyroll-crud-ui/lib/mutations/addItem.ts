import GummyrollIdl from "../../../target/idl/gummyroll.json";
import * as anchor from "@project-serum/anchor";
import { AnchorWallet } from "@solana/wallet-adapter-react";
import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";

export default async function addItem(
  anchorWallet: AnchorWallet,
  treeAccount: anchor.web3.PublicKey,
  data: string
) {
  const program = getGummyrollCrudProgram();
  const treeAdmin = anchorWallet.publicKey;
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );
  const txid = await program.methods
    .add(Buffer.from(data, "utf-8"))
    .accounts({
      authority: anchorWallet.publicKey,
      authorityPda,
      merkleRoll: treeAccount,
      gummyrollProgram: new anchor.web3.PublicKey(
        GummyrollIdl.metadata.address
      ),
    })
    .rpc({ commitment: "confirmed" });
  const transaction = await program.provider.connection.getTransaction(txid, {
    commitment: "confirmed",
  });
  try {
    let index = 0;
    transaction!.meta!.logMessages!.some((_message) => {
      // TODO: Actually trawl the transaction logs looking for the index.
      return true;
    });
    return index;
  } catch (e) {
    console.error(e);
    throw new Error("Could not find index of new asset");
  }
}
