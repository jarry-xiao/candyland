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
import getItemsForOwner from "../lib/loaders/getItemsForOwner";
import getItem from "../lib/loaders/getItem";

import * as styles from "../styles/app.css"; // Side-effectful import that adds global styles.
import "@solana/wallet-adapter-react-ui/styles.css"; // Side-effectful import to add styles for wallet modal.

/**
 * Temporary fetch implementation that knows how to look up mock data.
 * Eventually this will just be replaced with `fetch` and API URLs.
 */
async function localFetcher(...pathParts: string[]) {
  if (pathParts[0] === "item") {
    const [_, treeAccount, index] = pathParts;
    return await getItem(treeAccount, parseInt(index, 10));
  }
  if (pathParts[0] === "owner") {
    if (pathParts[2] === "items") {
      const ownerPubkey = pathParts[1];
      return await getItemsForOwner(ownerPubkey);
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
        fallback: serverData,
        fetcher: localFetcher,
        revalidateOnMount: false,
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
            <div className={styles.shell}>
              <SearchBar />
              <Component {...pageProps} />
            </div>
          </WalletModalProvider>
        </WalletProvider>
      </ConnectionProvider>
    </SWRConfig>
  );
}
