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
import { TreePayload } from "../../lib/loaders/getTreesForAuthority";

const AddAsset: NextPage = () => {
  const router = useRouter();
  const dataRef = React.createRef<HTMLTextAreaElement>();
  const [selectedTree, setSelectedTree] = useState<TreePayload | null>(null);
  const { mutate } = useSWRConfig();
  const anchorWallet = useAnchorWallet();
  if (!anchorWallet) {
    throw new Error("You must be logged in to create a new asset.");
  }
  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const targetTreeAccount = selectedTree;
    const data = dataRef.current?.value!;
    if (!targetTreeAccount) {
      return;
    }
    const indexOfNewAsset = await addAsset(
      new anchor.web3.PublicKey(selectedTree.account),
      new anchor.web3.PublicKey(selectedTree.authority),
      data
    );
    const nodeIndex = (1 << 14) + indexOfNewAsset;
    
    const newAssetPayload = {
      data,
      index: nodeIndex,
      owner: selectedTree.authority,
      treeAccount: targetTreeAccount.account,
      treeAdmin: targetTreeAccount.authority,
    };
    await Promise.all([
      mutate<AssetPayload>(
        unstable_serialize([
          "asset",
          targetTreeAccount.account,
          nodeIndex.toString(),
        ]),
        newAssetPayload
      ),
      mutate<AssetPayload[]>(
        unstable_serialize(["owner", selectedTree.authority, "assets"]),
        (currentAssets) => [...(currentAssets || []), newAssetPayload]
      ),
    ]);
    router.replace({
      pathname: "/asset/[treeAccount]/[index]",
      query: {
        treeAccount: targetTreeAccount.account,
        index: nodeIndex.toString(),
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
            onChange={setSelectedTree}
            value={selectedTree?.account}
          />
        </label>
        <label htmlFor="data">
          <p>Data</p>
          <textarea name="data" ref={dataRef}></textarea>
        </label>
        <p>
          <Button type="submit" disabled={!selectedTree}>
            Create Asset
          </Button>
        </p>
      </form>
    </>
  );
};

export default AddAsset;
