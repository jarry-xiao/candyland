import { BN, web3 } from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";
import React from "react";
import { hash } from "../../tests/merkle-tree";
import { PathNode, decodeMerkleRoll, OnChainMerkleRoll } from "../gummyroll";
import { NFTDatabaseConnection } from "./db";

export async function updateMerkleRollSnapshot(
  connection: web3.Connection,
  merkleRollKey: PublicKey,
  setMerkleRoll: any
) {
  const result = await connection.getAccountInfo(merkleRollKey, "confirmed");
  if (result) {
    setMerkleRoll(decodeMerkleRoll(result?.data));
  }
}

export async function updateMerkleRollLive(
  connection: web3.Connection,
  merkleRollKey: PublicKey,
  setMerkleRoll: any
) {
  let subId = connection.onAccountChange(
    merkleRollKey,
    (result) => {
      if (result) {
        try {
          setMerkleRoll(decodeMerkleRoll(result?.data));
        } catch (e) {
          console.log("Failed to deserialize account", e);
        }
      }
    },
    "confirmed"
  );
  return subId;
}

export async function getUpdatedBatch(
  merkleRoll: OnChainMerkleRoll,
  db: NFTDatabaseConnection
) {
  // If seq > max JS int it's all over :(
  const seq = merkleRoll.roll.sequenceNumber.toNumber();
  console.log(`Received Batch! Sequence=${seq}`);
  const pathNodes = merkleRoll.getChangeLogsWithNodeIndex();
  interface PathDict {
    [key: number]: PathNode;
  }
  let data: PathDict = {};
  for (const [i, path] of pathNodes.entries()) {
    data[seq - i] = path;
  }
  // TODO: make this atomic maybe / use caching to prevent too much duplication
  for (const [i, [seq, path]] of Object.entries(data).entries()) {
    db.upsertStmt.bind({
      "@node_idx": path.index,
      "@seq": seq,
      "@level": i,
      "@hash": path.node.toBase58(),
    });
    await db.upsertStmt.run();
  }
}
