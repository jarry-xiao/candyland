import * as anchor from "@project-serum/anchor";
import { NextPage } from "next";
import { useRouter } from "next/router";
import React, { FormEvent } from "react";
import Button from "../../../components/Button";
import addItem from "../../../lib/mutations/addItem";
import { useWallet } from "@solana/wallet-adapter-react";

const OwnerAddItem: NextPage = () => {
  const router = useRouter();
  const dataRef = React.createRef<HTMLTextAreaElement>();
  const treeAccountRef = React.createRef<HTMLInputElement>();
  const { publicKey } = useWallet();
  if (!publicKey) {
    throw new Error("You must be logged in to create a new asset.");
  }
  function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const treeAccount = new anchor.web3.PublicKey(
      treeAccountRef.current?.value!
    );
    addItem(treeAccount, dataRef.current?.value!);
  }
  return (
    <>
      <h1>Add item for {router.query.ownerPubkey}</h1>
      <form onSubmit={handleSubmit}>
        <label htmlFor="treeAccount">
          <p>Tree id</p>
          <input name="treeAccount" ref={treeAccountRef} type="text" />
        </label>
        <label htmlFor="data">
          <p>Data</p>
          <textarea name="data" ref={dataRef}></textarea>
        </label>
        <p>
          <Button type="submit">Add</Button>
        </p>
      </form>
    </>
  );
};

export default OwnerAddItem;
