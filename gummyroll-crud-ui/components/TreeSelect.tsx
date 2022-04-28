import React, { SyntheticEvent, useRef, useState } from "react";
import * as anchor from "@project-serum/anchor";
import Button from "./Button";
import useSWRImmutable from "swr/immutable";
import createTree from "../lib/mutations/createTree";
import { AnchorWallet } from "@solana/wallet-adapter-react";
import { TreePayload } from "../lib/loaders/getTreesForAuthority";

type Props = Readonly<
  Omit<React.ComponentProps<"select">, "onChange" | "value"> & {
    anchorWallet: AnchorWallet;
    onChange(tree: TreePayload | null): void;
    value?: string;
  }
>;

export default function TreeSelect({
  anchorWallet,
  onChange,
  value,
  ...selectProps
}: Props) {
  const { data: trees, mutate } = useSWRImmutable<TreePayload[]>([
    "owner",
    anchorWallet.publicKey.toBase58(),
    "trees",
  ]);
  async function handleCreateNewTree(
    e: SyntheticEvent<HTMLButtonElement, MouseEvent>
  ) {
    e.preventDefault();
    let newTree: TreePayload | null = null;
    await mutate(async (currentData) => {
      const account = await createTree(anchorWallet, 14, 64);
      newTree = {
        account: account.toBase58(),
        authority: anchorWallet.publicKey.toBase58(),
      };
      return [newTree, ...(currentData ?? [])];
    });
    onChange(newTree);
  }
  return (
    <>
      <select
        {...selectProps}
        disabled={trees?.length === 0}
        onChange={(e) => {
          onChange(
            e.target.value
              ? trees?.find((tree) => tree.account === e.target.value)!
              : null
          );
        }}
        value={value}
      >
        {trees?.length === 0 ? (
          <option value="">No trees</option>
        ) : (
          <option value="">Select a tree&hellip;</option>
        )}
        {trees?.map(({ account }) => {
          return (
            <option key={account} value={account}>
              {account}
            </option>
          );
        })}
      </select>
      <Button onClick={handleCreateNewTree}>Create new tree</Button>
    </>
  );
}
