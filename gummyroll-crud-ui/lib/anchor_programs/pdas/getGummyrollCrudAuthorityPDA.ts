import * as anchor from "@project-serum/anchor";
import getGummyrollCrudProgram from "../getGummyrollCrudProgram";

export default async function getGummyrollCrudAuthorityPDA(
  treeAddress: anchor.web3.PublicKey,
  treeAdmin: anchor.web3.PublicKey
) {
  return await anchor.web3.PublicKey.findProgramAddress(
    [
      Buffer.from("gummyroll-crud-authority-pda", "utf-8"),
      treeAddress.toBuffer(),
      treeAdmin.toBuffer(),
    ],
    getGummyrollCrudProgram().programId
  );
}
