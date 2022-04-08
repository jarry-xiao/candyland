import * as anchor from "@project-serum/anchor";
import { NextPage } from "next";
import { useRouter } from "next/router";
import React, { FormEvent, useState } from "react";
import Button from "../../components/Button";
import addItem from "../../lib/mutations/addItem";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import TreeSelect from "../../components/TreeSelect";
import { unstable_serialize, useSWRConfig } from "swr";
import { ItemPayload } from "../../lib/loaders/ItemTypes";

const AddItem: NextPage = () => {
  const router = useRouter();
  const dataRef = React.createRef<HTMLTextAreaElement>();
  const [selectedTreeAccount, setSelectedTreeAccount] =
    useState<anchor.web3.PublicKey | null>(null);
  const { mutate } = useSWRConfig();
  const anchorWallet = useAnchorWallet();
  if (!anchorWallet) {
    throw new Error("You must be logged in to create a new asset.");
  }
  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const targetTreeAccount = selectedTreeAccount;
    const newOwner = anchorWallet!.publicKey;
    const data = dataRef.current?.value!;
    if (!targetTreeAccount) {
      return;
    }
    const indexOfNewItem = await addItem(
      anchorWallet!,
      selectedTreeAccount,
      data
    );
    const newItemPayload = {
      data,
      index: indexOfNewItem,
      owner: newOwner.toBase58(),
      treeAccount: targetTreeAccount.toBase58(),
    };
    await Promise.all([
      mutate<ItemPayload>(
        unstable_serialize([
          "item",
          targetTreeAccount.toBase58(),
          indexOfNewItem.toString(),
        ]),
        newItemPayload
      ),
      mutate<ItemPayload[]>(
        unstable_serialize(["owner", newOwner.toBase58(), "items"]),
        (currentItems) => [...(currentItems || []), newItemPayload]
      ),
    ]);
    router.replace({
      pathname: "/item/[treeAccount]/[index]",
      query: {
        treeAccount: targetTreeAccount.toBase58(),
        index: indexOfNewItem.toString(),
      },
    });
  }
  return (
    <>
      <h1>Add item for {router.query.ownerPubkey}</h1>
      <form onSubmit={handleSubmit}>
        <label htmlFor="treeAccount">
          <p>Tree id</p>
          <TreeSelect
            anchorWallet={anchorWallet!}
            name="treeAccount"
            onChange={setSelectedTreeAccount}
            value={selectedTreeAccount}
          />
        </label>
        <label htmlFor="data">
          <p>Data</p>
          <textarea name="data" ref={dataRef}></textarea>
        </label>
        <p>
          <Button type="submit" disabled={!selectedTreeAccount}>
            Add
          </Button>
        </p>
      </form>
    </>
  );
};

export default AddItem;
