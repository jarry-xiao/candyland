import React, { SyntheticEvent } from "react";
import * as anchor from "@project-serum/anchor";
import Button from "./Button";
import useSWRImmutable from "swr/immutable";
import createTree from "../lib/mutations/createTree";
import { AnchorWallet } from "@solana/wallet-adapter-react";

type Props = Readonly<
  Omit<React.ComponentProps<"select">, "onChange" | "value"> & {
    anchorWallet: AnchorWallet;
    onChange(treeAccount: anchor.web3.PublicKey | null): void;
    value: anchor.web3.PublicKey | null;
  }
>;

export default function TreeSelect({
  anchorWallet,
  onChange,
  value,
  ...selectProps
}: Props) {
  const { data: treeAccounts, mutate } = useSWRImmutable<
    anchor.web3.PublicKey[]
  >(["owner", anchorWallet.publicKey.toBase58(), "trees"]);
  async function handleCreateNewTree(
    e: SyntheticEvent<HTMLButtonElement, MouseEvent>
  ) {
    e.preventDefault();
    let newTreeAccount: anchor.web3.PublicKey | null = null;
    await mutate(async (currentData) => {
      newTreeAccount = await createTree(anchorWallet, 14, 64);
      return [newTreeAccount, ...(currentData ?? [])];
    });
    onChange(newTreeAccount);
  }
  return (
    <>
      <select
        {...selectProps}
        disabled={treeAccounts?.length === 0}
        onChange={(e) => {
          onChange(
            e.target.value ? new anchor.web3.PublicKey(e.target.value) : null
          );
        }}
        value={value?.toBase58()}
      >
        {treeAccounts?.length === 0 ? (
          <option value="">No trees</option>
        ) : (
          <option value="">Select a tree&hellip;</option>
        )}
        {treeAccounts?.map((pubKey) => {
          const displayPubkey = pubKey.toBase58();
          return (
            <option key={displayPubkey} value={displayPubkey}>
              {displayPubkey}
            </option>
          );
        })}
      </select>
      <Button onClick={handleCreateNewTree}>Create new tree</Button>
    </>
  );
}
