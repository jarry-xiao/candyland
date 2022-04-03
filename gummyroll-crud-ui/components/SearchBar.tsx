import { useRouter } from "next/router";
import * as styles from "../styles/SearchBar.css";
import InputUnstyled from "@mui/base/InputUnstyled";
import React from "react";
import { useWalletModal } from "@solana/wallet-adapter-react-ui";
import { useWallet } from "@solana/wallet-adapter-react";
import Button from "./Button";
import Link from "next/link";

export default function SearchBar() {
  const router = useRouter();
  const inputRef = React.createRef<HTMLInputElement>();
  const { setVisible: showModal } = useWalletModal();
  const { disconnect, publicKey } = useWallet();
  return (
    <>
      <div className={styles.header}>
        <form
          className={styles.searchForm}
          onSubmit={(e) => {
            e.preventDefault();
            router.push({
              pathname: "/owner/[ownerPubkey]/items",
              query: { ownerPubkey: inputRef.current?.value },
            });
          }}
        >
          {/* @ts-ignore This type includes `ownerState` when it shouldn't. */}
          <InputUnstyled
            autoFocus
            placeholder="Search by pubkey&hellip;"
            componentsProps={{
              input: {
                className: styles.input,
                ref: inputRef,
              },
              root: {
                className: styles.inputRoot,
              },
            }}
          />
        </form>
        <div className={styles.accountControls}>
          {publicKey ? (
            <>
              <Link
                href={{
                  pathname: "/owner/[ownerPubkey]/items",
                  query: { ownerPubkey: publicKey.toString() },
                }}
                passHref
              >
                <Button title={publicKey.toString()}>My items</Button>
              </Link>
              <Button onClick={disconnect}>Disconnect</Button>
            </>
          ) : (
            <Button
              onClick={() => {
                showModal(true);
              }}
            >
              Connect wallet
            </Button>
          )}
        </div>
      </div>
      <p className={styles.inputHint}>
        Hint: Try <code>C2jDL4pcwpE2pP5EryTGn842JJUJTcurPGZUquQjySxK</code>
      </p>
    </>
  );
}
