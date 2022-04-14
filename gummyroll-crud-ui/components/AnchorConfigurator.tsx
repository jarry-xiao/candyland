import * as anchor from "@project-serum/anchor";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import React from "react";
import { useEffect } from "react";
import getAnchorConnection from "../lib/db/getAnchorConnection";

const AnchorConfigurator: React.FC = function AnchorConfigurator({ children }) {
  const anchorWallet = useAnchorWallet();
  useEffect(() => {
    if (!anchorWallet) {
      return;
    }
    const connection = getAnchorConnection();
    const provider = new anchor.Provider(
      connection,
      anchorWallet,
      anchor.Provider.defaultOptions()
    );
    anchor.setProvider(provider);
  }, [anchorWallet]);
  return <>{children}</>;
};

export default AnchorConfigurator;
