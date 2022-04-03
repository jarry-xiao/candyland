import { NextPage } from "next";
import { useRouter } from "next/router";
import React, { FormEvent } from "react";
import Button from "../../../components/Button";

const OwnerAddItem: NextPage = () => {
  const router = useRouter();
  const dataRef = React.createRef<HTMLTextAreaElement>();
  const treeIdRef = React.createRef<HTMLInputElement>();
  function handleSubmit(e: FormEvent) {
    e.preventDefault();
  }
  return (
    <>
      <h1>Add item for {router.query.ownerPubkey}</h1>
      <form onSubmit={handleSubmit}>
        <label htmlFor="treeId">
          <p>Tree id</p>
          <input name="treeId" ref={treeIdRef} type="text" />
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
