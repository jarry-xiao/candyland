import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import { Program, Provider, } from "@project-serum/anchor";
import {
  Connection as web3Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import { assert } from "chai";

import * as Collections from 'typescript-collections';

import { buildTree, getProofOfLeaf, updateTree, Tree, checkProofHash } from "./merkle-tree";
import { decodeMerkleRoll, getMerkleRollAccountSize } from "./merkle-roll-serde";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import fetch from "node-fetch";
import { sleep } from "@metaplex-foundation/amman/dist/utils";

// @ts-ignore
const Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;

export function chunk<T>(array: T[], size: number): Collections.Stack<T[]> {
  const arr = array.reverse();
  const chunked = Array.from({ length: Math.ceil(arr.length / size) }, (_: any, i: number) =>
    arr.slice(i * size, i * size + size + 1)
  );

  const queue = new Collections.Stack<T[]>();
  chunked.map((chunk) => queue.add(chunk))
  return queue;
}

describe("gummyroll-continuous-fetchproof", () => {
  let connection: web3Connection;
  let wallet: NodeWallet;
  let offChainTree: ReturnType<typeof buildTree>;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;
  anchor.setProvider(anchor.Provider.env());

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;
  // This is hardware dependent... if too large, then majority of tx's will fail to confirm
  const BATCH_SIZE = 5;

  async function createEmptyTreeOnChain(
    payer: Keypair
  ): Promise<Keypair> {
    const merkleRollKeypair = Keypair.generate();
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
    const allocAccountIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: merkleRollKeypair.publicKey,
      lamports:
        await Gummyroll.provider.connection.getMinimumBalanceForRentExemption(
          requiredSpace
        ),
      space: requiredSpace,
      programId: Gummyroll.programId,
    });

    const initGummyrollIx = Gummyroll.instruction.initEmptyGummyroll(
      MAX_DEPTH,
      MAX_SIZE,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
      }
    );

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await Gummyroll.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "singleGossip",
    });
    return merkleRollKeypair
  }

  function createEmptyTreeOffChain(): Tree {
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    let tree = buildTree(leaves);
    return tree;
  }

  beforeEach(async () => {
    payer = Keypair.generate();
    connection = new web3Connection(
      "http://localhost:8899",
      {
        commitment: 'singleGossip'
      }
    );
    wallet = new NodeWallet(payer)
    anchor.setProvider(new Provider(connection, wallet, { commitment: connection.commitment, skipPreflight: true }));
    await Gummyroll.provider.connection.confirmTransaction(
      await Gummyroll.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );

    merkleRollKeypair = await createEmptyTreeOnChain(payer);

    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );

    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    // Check header bytes are set correctly
    assert(onChainMerkle.header.maxDepth === MAX_DEPTH, `Max depth does not match ${onChainMerkle.header.maxDepth}, expected ${MAX_DEPTH}`);
    assert(onChainMerkle.header.maxBufferSize === MAX_SIZE, `Max buffer size does not match ${onChainMerkle.header.maxBufferSize}, expected ${MAX_SIZE}`);

    assert(
      onChainMerkle.header.authority.equals(payer.publicKey),
      "Failed to write auth pubkey"
    );

    offChainTree = createEmptyTreeOffChain();

    assert(
      onChainMerkle.roll.changeLogs[0].root.equals(new PublicKey(offChainTree.root)),
      "On chain root does not match root passed in instruction"
    );
  });

  // Will be used in future test
  function createReplaceIx(merkleRollKeypair: Keypair, payer: Keypair, i: number, nodeProof: any, root: any, oldLeaf: any) {
    /// Empty nodes are special, so we have to create non-zero leaf for index 0
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([(MAX_SIZE - i + 1) + 1])));

    const replaceLeafIx = Gummyroll.instruction.replaceLeaf(
      { inner: root },
      { inner: oldLeaf },
      { inner: Array.from(newLeaf) },
      i,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: nodeProof,
      }
    );
    return replaceLeafIx;
  }

  function createInsertOrAppendIx(tree: Tree, merkleRollKeypair: Keypair, payer: Keypair, i: number) {
    /// Empty nodes are special, so we have to create non-zero leaf for index 0
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + i])));
    let nodeProof = getProofOfLeaf(tree, i).map((node) => { return { pubkey: new PublicKey(node.node), isSigner: false, isWritable: false } });
    return Gummyroll.instruction.insertOrAppend(
      { inner: Array.from(tree.root) },
      { inner: Array.from(newLeaf) },
      i,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: nodeProof,
      }
    );
  }

  function createAppend(merkleRollKeypair: Keypair, payer: Keypair, i: number) {
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + i])));
    return Gummyroll.instruction.append(
      { inner: Array.from(newLeaf) },
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
      }
    );
  }

  it(`${MAX_SIZE} transactions in batches of ${BATCH_SIZE}, querying for proofs every ${MAX_SIZE}`, async () => {
    // first fill up MAX DEPTH appends
    let appendsLeft = [];
    for (let i = 0; i < MAX_SIZE; i++) { appendsLeft.push(i) }

    while (appendsLeft.length) {
      console.log(`Appending ${appendsLeft.length} entries to the tree without knowing their indices`);
      const toConfirm = [];
      const txIdToIndex: { [key: string]: number } = {};
      for (const i of appendsLeft) {
        const tx = new Transaction().add(createAppend(merkleRollKeypair, payer, i))
        tx.feePayer = payer.publicKey;
        tx.recentBlockhash = (
          await connection.getLatestBlockhash('singleGossip')
        ).blockhash;

        await wallet.signTransaction(tx);
        const rawTx = tx.serialize();
        const txId = await connection.sendRawTransaction(rawTx, { skipPreflight: true });
        txIdToIndex[txId] = i;
        toConfirm.push(txId);
        await sleep(100);
      }

      const txsToReplay = [];
      await Promise.all(toConfirm.map(async (txId) => {
        const confirmation = await connection.confirmTransaction(txId, "confirmed")
        if (confirmation.value.err && txIdToIndex[txId]) {
          txsToReplay.push(txIdToIndex[txId]);
        }
        return confirmation;
      }));
      await sleep(500);

      appendsLeft = txsToReplay;
    }

    let merkleRollInfo = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );

    let onChainRoll = decodeMerkleRoll(merkleRollInfo.data);

    let synced = false;
    while (!synced) {
      let result = await fetch("http://localhost:3000/changesProcessed").then((resp) => resp.json())
      console.log(result);
      if (result.numChanges === MAX_SIZE) { synced = true };
      await sleep(900);
    }

    // -- (TODO: ngundotra) Add check to verify that the off-chain tree's root matches on-chain root by this point
    // merkleRollInfo = await Gummyroll.provider.connection.getAccountInfo(
    //   merkleRollKeypair.publicKey
    // );
    // onChainRoll = decodeMerkleRoll(merkleRollInfo.data);
    // console.log("On chain root:", new PublicKey(onChainRoll.roll.changeLogs[onChainRoll.roll.activeIndex].root).toString());

    console.log(`Requesting batch of ${MAX_SIZE} proofs`);
    let replacesToSend = [];
    for (let i = 0; i < MAX_SIZE; i++) {
      try {
        const replaceProof = await fetch(`http://localhost:3000/proof?leafIndex=${i}`)
          .then((resp) => {
            return resp.json()
          });

        if (!checkProofHash(replaceProof.proof, replaceProof.root, replaceProof.leaf, i)) {
          console.log("proof for index was incorrect:", i);
        }

        const mappedProof = replaceProof.proof.map((pubkeyBytes) => ({
          pubkey: new PublicKey(Buffer.from(Uint8Array.from(pubkeyBytes))),
          isSigner: false,
          isWritable: false,
        }));

        const replaceIx = createReplaceIx(merkleRollKeypair, payer, i,
          mappedProof,
          replaceProof.root,
          Buffer.from(Uint8Array.from(replaceProof.leaf)),
        );
        replacesToSend.push(replaceIx);
      } catch (e) {
        console.log("Error fetching proof for index:", i, "; skipping");
        continue;
      }
    };
    console.log(`Successfully retrieved ${replacesToSend.length} proofs`);

    while (replacesToSend.length > 0) {
      const txIds = [];
      const txIdToIndex: Record<string, TransactionInstruction> = {};
      for (const replaceIx of replacesToSend) {
        const promise = async () => {
          const tx = new Transaction().add(replaceIx);

          tx.feePayer = payer.publicKey;
          tx.recentBlockhash = (
            await connection.getLatestBlockhash('singleGossip')
          ).blockhash;

          await wallet.signTransaction(tx);
          const rawTx = tx.serialize();
          return connection.sendRawTransaction(rawTx, { skipPreflight: true })
            .then((txId) => {
              txIdToIndex[txId] = replaceIx;
              return txId;
            })
            .catch((reason) => {
              console.error(reason);
              return replaceIx;
            })
        };

        txIds.push(promise());
      }

      const sendResults: (string | TransactionInstruction)[] = (await Promise.all(txIds));
      // console.log("send results:", sendResults);
      await sleep(12000);

      const batchToConfirm = sendResults.filter((result) => typeof result === "string") as string[];
      const txsToReplay = sendResults.filter((err) => typeof err !== "string") as TransactionInstruction[];

      await Promise.all(batchToConfirm.map(async (txId) => {
        const confirmation = await connection.confirmTransaction(txId, "confirmed")
        if (confirmation.value.err && txIdToIndex[txId]) {
          txsToReplay.push(txIdToIndex[txId]);
        }
        return confirmation;
      }));

      if (txsToReplay.length) {
        // batchesToSend.add(txsToReplay);
        console.log(`${txsToReplay.length} tx's failed in batch`)
      }
      replacesToSend = txsToReplay;
    }

    // indicesToSend = indicesLeft;

  });
});
