import GummyrollIdl from "../../../target/idl/gummyroll.json";
import * as anchor from "@project-serum/anchor";

export type TreePayload = Readonly<{
  account: string;
  authority: string;
}>;

export default async function getTreesForAuthority(
  authority: string
): Promise<TreePayload[]> {
  const result = await anchor
    .getProvider()
    .connection.getParsedProgramAccounts(
      new anchor.web3.PublicKey(GummyrollIdl.metadata.address),
      "confirmed"
    );
  return result.map((result) => ({
    account: result.pubkey.toBase58(),
    authority: authority,
  }));
}
