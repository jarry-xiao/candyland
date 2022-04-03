import { useRouter } from "next/router";
import * as styles from "../styles/SearchBar.css";
import InputUnstyled, { InputOwnerState } from "@mui/base/InputUnstyled";
import React from "react";

export default function SearchBar() {
  const router = useRouter();
  const inputRef = React.createRef<HTMLInputElement>();
  return (
    <form
      className={styles.header}
      onSubmit={(e) => {
        e.preventDefault();
        router.push({
          pathname: "/owner/[ownerPubkey]/items",
          query: { ownerPubkey: inputRef.current?.value },
        });
      }}
    >
      <InputUnstyled
        autoFocus
        placeholder="Search for items by owner pubkey&hellip;"
        componentsProps={{
          // @ts-ignore This type includes `ownerState` when it shouldn't.
          input: {
            className: styles.input,
            ref: inputRef,
          },
        }}
      />
      <p className={styles.inputHint}>
        Hint: Try <code>aJ69C1ZjyGM2eeZknnkEQ6hjA48dKCIyqfoZaHXZFDz</code>
      </p>
    </form>
  );
}
