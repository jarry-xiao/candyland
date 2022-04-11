import * as anchor from "@project-serum/anchor";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import React from "react";
import { useEffect } from "react";

const AnchorConfigurator: React.FC = function AnchorConfigurator({ children }) {
  const anchorWallet = useAnchorWallet();
  useEffect(() => {
    if (!anchorWallet) {
      return;
    }
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
    const provider = new anchor.Provider(
      new anchor.web3.Connection(endpoint),
      anchorWallet,
      anchor.Provider.defaultOptions()
    );
    anchor.setProvider(provider);
  }, [anchorWallet]);
  return <>{children}</>;
};

export default AnchorConfigurator;
