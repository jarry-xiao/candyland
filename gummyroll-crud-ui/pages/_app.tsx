import type { AppProps } from "next/app";
import SearchBar from "../components/SearchBar";

import * as styles from "../styles/app.css"; // Side-effectful import that adds global styles.

export default function MyApp({ Component, pageProps }: AppProps) {
  return (
    <div className={styles.shell}>
      <SearchBar />
      <Component {...pageProps} />
    </div>
  );
}
