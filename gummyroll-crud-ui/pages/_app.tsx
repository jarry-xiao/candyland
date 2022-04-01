import type { AppProps } from "next/app";
import Head from "next/head";
import SearchBar from "../components/SearchBar";

import * as styles from "../styles/app.css"; // Side-effectful import that adds global styles.

export default function MyApp({ Component, pageProps }: AppProps) {
  return (
    <>
      <Head>
        <meta
          name="viewport"
          content="initial-scale=1.0, width=device-width, user-scalable=no"
        />
      </Head>
      <div className={styles.shell}>
        <SearchBar />
        <Component {...pageProps} />
      </div>
    </>
  );
}
