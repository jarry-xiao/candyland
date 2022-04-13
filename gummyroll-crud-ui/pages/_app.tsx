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
import getAssetsForOwner from "../lib/loaders/getAssetsForOwner";
import getAsset from "../lib/loaders/getAsset";
import AnchorConfigurator from "../components/AnchorConfigurator";

import * as styles from "../styles/app.css"; // Side-effectful import that adds global styles.
import "@solana/wallet-adapter-react-ui/styles.css"; // Side-effectful import to add styles for wallet modal.
import getTreesForAuthority from "../lib/loaders/getTreesForAuthority";

/**
 * Temporary fetch implementation that knows how to look up mock data.
 * Eventually this will just be replaced with `fetch` and API URLs.
 */
async function localFetcher(...pathParts: string[]) {
  if (pathParts[0] === "asset") {
    const [_, treeAccount, index] = pathParts;
    return await getAsset(treeAccount, parseInt(index, 10));
  }
  if (pathParts[0] === "owner") {
    const ownerPubkey = pathParts[1];
    if (pathParts[2] === "assets") {
      return await getAssetsForOwner(ownerPubkey);
    } else if (pathParts[2] === "trees") {
      return await getTreesForAuthority(ownerPubkey);
    }
  }
}

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
        fetcher: process.env.NEXT_PUBLIC_TREE_SERVER_API_ENDPOINT
          ? async (...pathParts: string[]) => {
              const url = new URL(
                pathParts.join("/") /* relative path */,
                process.env.NEXT_PUBLIC_TREE_SERVER_API_ENDPOINT /* base */
              );
              const response = await fetch(url.toString());
              if (!response.ok) {
                throw new Error(response.statusText);
              }
              const { data } = await response.json();
              return data;
            }
          : localFetcher,
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
