import GummyrollIdl from "../../../target/idl/gummyroll.json";
import * as anchor from "@project-serum/anchor";
import { AnchorWallet } from "@solana/wallet-adapter-react";
import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";

type MaxDepth = 14 | 16 | 18 | 20 | 22;
type MaxBufferSize = 64 | 128 | 256 | 1024 | 2448;

export default async function createTree(
  anchorWallet: AnchorWallet,
  maxDepth: MaxDepth,
  maxBufferSize: MaxBufferSize
) {
  const program = getGummyrollCrudProgram();
  const gummyrollProgramId = new anchor.web3.PublicKey(
    GummyrollIdl.metadata.address
  );
  const treeAdmin = anchorWallet.publicKey;
  const treeAccountSeed = Date.now().toString();
  const treeAccount = await anchor.web3.PublicKey.createWithSeed(
    treeAdmin,
    treeAccountSeed,
    gummyrollProgramId
  );
  const requiredSpace = getMerkleRollAccountSize(maxDepth, maxBufferSize);
  const allocGummyrollAccountIx =
    anchor.web3.SystemProgram.createAccountWithSeed({
      basePubkey: treeAdmin,
      fromPubkey: treeAdmin,
      newAccountPubkey: treeAccount,
      lamports:
        await program.provider.connection.getMinimumBalanceForRentExemption(
          requiredSpace
        ),
      seed: treeAccountSeed,
      space: requiredSpace,
      programId: gummyrollProgramId,
    });
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );
  const txid = await program.methods
    .createTree(maxDepth, maxBufferSize)
    .preInstructions([allocGummyrollAccountIx])
    .accounts({
      authority: treeAdmin,
      authorityPda,
      merkleRoll: treeAccount,
      gummyrollProgram: gummyrollProgramId,
    })
    .rpc({ commitment: "confirmed" });
  return treeAccount;
}

/**
 * FIXME: This is copypasta from elsewhere.
 * Publish this properly and import it for use.
 */
export function getMerkleRollAccountSize(
  maxDepth: number,
  maxBufferSize: number
): number {
  let headerSize = 8 + 32;
  let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
  let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
  let merkleRollSize = 8 + 8 + changeLogSize + rightMostPathSize;
  return merkleRollSize + headerSize;
}
