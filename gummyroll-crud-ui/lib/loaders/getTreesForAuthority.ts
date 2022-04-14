import * as anchor from "@project-serum/anchor";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

export type TreePayload = Readonly<{
  account: string;
  authority: string;
}>;

export default async function getTreesForAuthority(
  authority: string
): Promise<TreePayload[]> {
  const endpointOrCluster: string | anchor.web3.Cluster =
    process.env.NEXT_PUBLIC_RPC_ENDPOINT_OR_CLUSTER!;
  let endpoint: string;
  try {
    endpoint = anchor.web3.clusterApiUrl(
      endpointOrCluster as anchor.web3.Cluster,
      true /* tls */
    );
  } catch {
    endpoint = endpointOrCluster as string;
  }
  const result = await new anchor.web3.Connection(
    endpoint
  ).getParsedProgramAccounts(GummyrollProgramId, "confirmed");
  return result.map((result) => ({
    account: result.pubkey.toBase58(),
    authority: authority,
  }));
}
