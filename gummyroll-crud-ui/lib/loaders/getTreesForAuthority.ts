import * as anchor from "@project-serum/anchor";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

export type TreePayload = Readonly<{
  account: string;
  authority: string;
}>;

export default async function getTreesForAuthority(
  authority: string
): Promise<TreePayload[]> {
  const result = await anchor
    .getProvider()
    .connection.getParsedProgramAccounts(GummyrollProgramId, "confirmed");
  return result.map((result) => ({
    account: result.pubkey.toBase58(),
    authority: authority,
  }));
}
