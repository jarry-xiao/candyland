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
    const provider = new anchor.Provider(
      new anchor.web3.Connection("http://localhost:8899"),
      anchorWallet,
      anchor.Provider.defaultOptions()
    );
    anchor.setProvider(provider);
  }, [anchorWallet]);
  return <>{children}</>;
};

export default AnchorConfigurator;
