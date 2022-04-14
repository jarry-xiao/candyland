import * as anchor from "@project-serum/anchor";

let connection: anchor.web3.Connection;
export default function getAnchorConnection() {
  if (!connection) {
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
    connection = new anchor.web3.Connection(endpoint);
  }
  return connection;
}
