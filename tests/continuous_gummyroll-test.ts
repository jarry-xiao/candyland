import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import { Program } from "@project-serum/anchor";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";

import { buildTree, getProofOfLeaf, updateTree, Tree } from "./merkle-tree";
import { decodeMerkleRoll, getMerkleRollAccountSize} from "./merkle-roll-serde";
import { sleep } from "../deps/metaplex-program-library/metaplex/js/test/utils";

// @ts-ignore
const Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;

describe("gummyroll-continuous", () => {
  anchor.setProvider(anchor.Provider.env());
  let offChainTree: ReturnType<typeof buildTree>;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;

  const MAX_SIZE = 1024;
  const MAX_DEPTH = 20;
  // This is hardware dependent... if too large, then majority of tx's will fail to confirm
  const BATCH_SIZE = 12;

  async function createEmptyTreeOnChain(
    payer: Keypair
  ): Promise<Keypair> {
    const merkleRollKeypair = Keypair.generate();
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
    const allocAccountIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: merkleRollKeypair.publicKey,
      lamports:
        await program.provider.connection.getMinimumBalanceForRentExemption(
          requiredSpace
        ),
      space: requiredSpace,
      programId: program.programId,
    });

    const initGummyrollIx = Gummyroll.instruction.initEmptyGummyroll(
      MAX_DEPTH,
      MAX_SIZE,
      root,
      leaf,
      tree.leaves.length,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: proof,
      }
    );

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await program.provider.send(tx, [payer, merkleRollKeypair], {
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

    assert(
      onChainMerkle.roll.changeLogs[0].root.equals(new PublicKey(tree.root)),
      "On chain root does not match root passed in instruction"
    );
  });

  // Will be used in future test
  function createReplaceIx(tree: Tree, merkleRollKeypair: Keypair, payer: Keypair, i: number) {
    /// Empty nodes are special, so we have to create non-zero leaf for index 0
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + i])));
    let nodeProof = getProofOfLeaf(tree, i).map((node) => { return { pubkey: new PublicKey(node.node), isSigner: false, isWritable: false } });
    const replaceLeafIx = Gummyroll.instruction.replaceLeaf(
      { inner: Array.from(tree.root) },
      { inner: Array.from(tree.leaves[i].node) },
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

  it("Continuous updating and syncing", async () => {
    let indicesToSend = [];
    for (let i = 0; i < MAX_SIZE; i++) {
      indicesToSend.push(i);
    };

    let lastActiveIndex = 0;
    while (indicesToSend.length > 0) {
      console.log(`Sending ${indicesToSend.length} transactions in batches of ${BATCH_SIZE}`);
      let batchesToSend = chunk<number>(indicesToSend, BATCH_SIZE);
      let indicesLeft: number[] = [];

      for (const batch of batchesToSend) {
        const txIds = [];
        const txIdToIndex: Record<string, number> = {};
        for (const i of batch) {
          const tx = new Transaction().add(createReplaceIx(offChainTree, merkleRollKeypair, payer, i));

          tx.feePayer = payer.publicKey;
          tx.recentBlockhash = (
            await connection.getLatestBlockhash('singleGossip')
          ).blockhash;

          await wallet.signTransaction(tx);
          const rawTx = tx.serialize();

          txIds.push(
            connection.sendRawTransaction(rawTx, { skipPreflight: true })
              .then((txId) => {
                txIdToIndex[txId] = i;
                return txId
              })
              .catch((reason) => {
                console.error(reason);
                return i
              })
          );
        }
        const sendResults: (string | number)[] = (await Promise.all(txIds));
        const batchToConfirm = sendResults.filter((result) => typeof result === "string") as string[];
        const txsToReplay = sendResults.filter((err) => typeof err === "number") as number[];
        if (txsToReplay.length) {
          indicesLeft = indicesLeft.concat(txsToReplay as number[]);
          console.log(`${txsToReplay.length} tx's failed in batch`)
        }

        // console.log("confirming batch");
        const confirmations = await Promise.all(batchToConfirm.map(async (txId) => {
          const confirmation = await connection.confirmTransaction(txId, "confirmed")
          if (confirmation.value.err && txIdToIndex[txId]) {
            txsToReplay.push(txIdToIndex[txId]);
          }
          return confirmation;
        }));
        // console.log(confirmations);

        const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
          merkleRollKeypair.publicKey
        );
        let onChainMerkle = decodeMerkleRoll(merkleRoll.data);
        // console.log("Active index:", onChainMerkle.roll.activeIndex);
        lastActiveIndex = onChainMerkle.roll.activeIndex;
        indicesLeft = indicesLeft.concat(txsToReplay);
      }

      indicesToSend = indicesLeft;
    }

    // Sync off-chain tree
    for (const i of indicesToSync) {
      updateTree(offChainTree, Buffer.alloc(32, Buffer.from(Uint8Array.from([1 + i]))), i);
    }

    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
        merkleRollKeypair.publicKey
    );
    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);
    
    const onChainRoot = onChainMerkle.roll.changeLogs[onChainMerkle.roll.activeIndex].root;
    const treeRoot = new PublicKey(offChainTree.root);
    console.log("onChainRoot:", onChainRoot.toString());
    console.log("offChainRoot:", treeRoot.toString());
    console.log("Active index:", onChainMerkle.roll.activeIndex);
    console.log("Buffer size:", onChainMerkle.roll.bufferSize);

    assert(
        onChainRoot.equals(treeRoot),
        "On chain root does not match root passed in instruction"
    );
  });
});
