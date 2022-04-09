import * as anchor from "@project-serum/anchor";
import { NextPage } from "next";
import { useRouter } from "next/router";
import React, { FormEvent, useState } from "react";
import Button from "../../components/Button";
import addAsset from "../../lib/mutations/addAsset";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import TreeSelect from "../../components/TreeSelect";
import { unstable_serialize, useSWRConfig } from "swr";
import { AssetPayload } from "../../lib/loaders/AssetTypes";

const AddAsset: NextPage = () => {
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
    const indexOfNewAsset = await addAsset(
      anchorWallet!,
      selectedTreeAccount,
      data
    );
    const newAssetPayload = {
      data,
      index: indexOfNewAsset,
      owner: newOwner.toBase58(),
      treeAccount: targetTreeAccount.toBase58(),
    };
    await Promise.all([
      mutate<AssetPayload>(
        unstable_serialize([
          "asset",
          targetTreeAccount.toBase58(),
          indexOfNewAsset.toString(),
        ]),
        newAssetPayload
      ),
      mutate<AssetPayload[]>(
        unstable_serialize(["owner", newOwner.toBase58(), "assets"]),
        (currentAssets) => [...(currentAssets || []), newAssetPayload]
      ),
    ]);
    router.replace({
      pathname: "/asset/[treeAccount]/[index]",
      query: {
        treeAccount: targetTreeAccount.toBase58(),
        index: indexOfNewAsset.toString(),
      },
    });
  }
  return (
    <>
      <h1>Add asset for {router.query.ownerPubkey}</h1>
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
            Create Asset
          </Button>
        </p>
      </form>
    </>
  );
};

export default AddAsset;
