import GummyrollIdl from "../../target/idl/gummyroll.json";
import * as anchor from "@project-serum/anchor";
import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import {
  PhantomWalletAdapter,
  SolletWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import type { AppProps } from "next/app";
import Head from "next/head";
import { useMemo } from "react";
import SearchBar from "../components/SearchBar";
import { SWRConfig } from "swr";
import AnchorConfigurator from "../components/AnchorConfigurator";

import * as styles from "../styles/app.css"; // Side-effectful import that adds global styles.
import "@solana/wallet-adapter-react-ui/styles.css"; // Side-effectful import to add styles for wallet modal.

export default function MyApp({
  Component,
  pageProps: { serverData, ...pageProps },
}: AppProps) {
  const wallets = useMemo(
    () => [new PhantomWalletAdapter(), new SolletWalletAdapter()],
    []
  );
  return (
    <SWRConfig
      value={{
        ...(serverData ? { fallback: serverData } : null),
        fetcher: async (...pathParts: string[]) => {
          const url = "/api/" + pathParts.join("/");
          const response = await fetch(url);
          if (!response.ok) {
            throw new Error(response.statusText);
          }
          const { data } = await response.json();
          return data;
        },
      }}
    >
      <Head>
        <meta
          name="viewport"
          content="initial-scale=1.0, width=device-width, user-scalable=no"
        />
      </Head>
      <ConnectionProvider endpoint="https://localhost:8899">
        <WalletProvider wallets={wallets} autoConnect>
          <WalletModalProvider>
            <AnchorConfigurator>
              <div className={styles.shell}>
                <SearchBar />
                <Component {...pageProps} />
              </div>
            </AnchorConfigurator>
          </WalletModalProvider>
        </WalletProvider>
      </ConnectionProvider>
    </SWRConfig>
  );
}
