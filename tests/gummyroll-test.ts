import * as anchor from "@project-serum/anchor";
import { BN, TransactionNamespace, InstructionNamespace } from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  TransactionInstruction
} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from 'crypto';

import { buildTree, hash, getProofOfLeaf, updateTree, Tree } from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
} from "./merkle-roll-serde";
import { logTx } from "./utils";

// @ts-ignore
const Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  let offChainTree: Tree;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;

  async function createTreeOnChain(
    payer: Keypair,
    numLeaves: number,
  ): Promise<[Keypair, Tree]> {
    const merkleRollKeypair = Keypair.generate();

    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    for (let i = 0; i < numLeaves; i++) {
      leaves[i] = crypto.randomBytes(32);
    }
    const tree = buildTree(leaves);

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

    const root = { inner: Array.from(tree.root) };
    const leaf = { inner: Array.from(leaves[numLeaves - 1]) };
    const proof = getProofOfLeaf(tree, numLeaves - 1).map((node) => {
      return {
        pubkey: new PublicKey(node.node),
        isSigner: false,
        isWritable: false,
      };
    });

    const initGummyrollIx = Gummyroll.instruction.initGummyrollWithRoot(
      MAX_DEPTH,
      MAX_SIZE,
      root,
      leaf,
      numLeaves - 1,
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
    await Gummyroll.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
    });
    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );

    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    // Check header bytes are set correctly
    assert(
      onChainMerkle.header.maxDepth === MAX_DEPTH,
      `Max depth does not match ${onChainMerkle.header.maxDepth}, expected ${MAX_DEPTH}`
    );
    assert(
      onChainMerkle.header.maxBufferSize === MAX_SIZE,
      `Max buffer size does not match ${onChainMerkle.header.maxBufferSize}, expected ${MAX_SIZE}`
    );

    assert(
      onChainMerkle.header.authority.equals(payer.publicKey),
      "Failed to write auth pubkey"
    );

    assert(
      onChainMerkle.roll.changeLogs[0].root.equals(new PublicKey(tree.root)),
      "On chain root does not match root passed in instruction"
    );

    return [merkleRollKeypair, tree]
  }

  function createReplaceIx(
    previousLeaf: Buffer,
    newLeaf: Buffer,
    index: number,
    offChainTree: Tree,
    merkleTreeKey: PublicKey,
    payer: Keypair,
    minimizeProofLength: boolean = false,
    treeHeight: number = -1,
  ): TransactionInstruction {
    const proof = getProofOfLeaf(offChainTree, index, minimizeProofLength, treeHeight);

    const nodeProof = proof.map((offChainTreeNode) => {
      return {
        pubkey: new PublicKey(offChainTreeNode.node),
        isSigner: false,
        isWritable: false,
      };
    });

    return Gummyroll.instruction.replaceLeaf(
      { inner: Array.from(offChainTree.root) },
      { inner: Array.from(previousLeaf) },
      { inner: Array.from(newLeaf) },
      index,
      {
        accounts: {
          merkleRoll: merkleTreeKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: nodeProof,
      }
    );
  }

  beforeEach(async () => {
    payer = Keypair.generate();

    await Gummyroll.provider.connection.confirmTransaction(
      await Gummyroll.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
  });

  describe("Having created a tree with a single leaf", () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(payer, 1);
    });
    it("Append single leaf", async () => {
      const newLeaf = hash(
        payer.publicKey.toBuffer(),
        payer.publicKey.toBuffer()
      );

      const appendIx = Gummyroll.instruction.append(
        { inner: Array.from(newLeaf) },
        {
          accounts: {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
          },
          signers: [payer],
        }
      );

      const tx = new Transaction().add(appendIx);
      const txid = await Gummyroll.provider.send(tx, [payer], {
        commitment: "confirmed",
      });
      await logTx(Gummyroll.provider, txid, false);

      updateTree(offChainTree, newLeaf, 1);

      const merkleRollAccount = await Gummyroll.provider.connection.getAccountInfo(
        merkleRollKeypair.publicKey
      );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
    it("Replace that leaf", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;

      const replaceLeafIx = createReplaceIx(previousLeaf, newLeaf, index, offChainTree, merkleRollKeypair.publicKey, payer);
      assert(replaceLeafIx.keys.length == (2 + 20), `Failed to create proof for ${MAX_DEPTH}`);

      const tx = new Transaction().add(replaceLeafIx);
      const txid = await Gummyroll.provider.send(tx, [payer], {
        commitment: "confirmed",
      });
      await logTx(Gummyroll.provider, txid, false);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount = await Gummyroll.provider.connection.getAccountInfo(
        merkleRollKeypair.publicKey
      );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });

    it("Replace that leaf with a minimal proof", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;

      const replaceLeafIx = createReplaceIx(previousLeaf,
        newLeaf,
        index,
        offChainTree,
        merkleRollKeypair.publicKey,
        payer,
        true,
        1
      );
      assert(replaceLeafIx.keys.length == (2 + 1), "Failed to minimize proof to expected size of 1");
      const tx = new Transaction().add(replaceLeafIx);
      const txid = await Gummyroll.provider.send(tx, [payer], {
        commitment: "confirmed",
      });
      await logTx(Gummyroll.provider, txid, false);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount = await Gummyroll.provider.connection.getAccountInfo(
        merkleRollKeypair.publicKey
      );
      const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root matches root of updated off chain tree"
      );
    });
  });

  describe(`Having created a tree with ${MAX_SIZE} leaves`, () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(payer, MAX_SIZE);
    });
    it(`Replace all of them in a block`, async () => {
      // Replace 64 leaves before syncing off-chain tree with on-chain tree

      // Cache all proofs so we can execute in single block
      let ixArray = [];
      let txList = [];

      const leavesToUpdate = [];
      for (let i = 0; i < MAX_SIZE; i++) {
        const index = i;
        const newLeaf = hash(
          payer.publicKey.toBuffer(),
          Buffer.from(new BN(i).toArray())
        );
        leavesToUpdate.push(newLeaf);
        const proof = getProofOfLeaf(offChainTree, index);

        const nodeProof = proof.map((offChainTreeNode) => {
          return {
            pubkey: new PublicKey(offChainTreeNode.node),
            isSigner: false,
            isWritable: false,
          };
        });
        const replaceIx = Gummyroll.instruction.replaceLeaf(
          { inner: Array.from(offChainTree.root) },
          { inner: Array.from(offChainTree.leaves[i].node) },
          { inner: Array.from(newLeaf) },
          index,
          {
            accounts: {
              merkleRoll: merkleRollKeypair.publicKey,
              authority: payer.publicKey,
            },
            signers: [payer],
            remainingAccounts: nodeProof,
          }
        );
        ixArray.push(replaceIx);
      };

      // Execute all replaces in a "single block"
      ixArray.map((ix) => {
        const tx = new Transaction().add(ix);
        txList.push(
          Gummyroll.provider.send(tx, [payer], {
            commitment: "confirmed",
            skipPreflight: true,
          })
        );
      });
      await Promise.all(txList);

      leavesToUpdate.map((leaf, index) => {
        updateTree(offChainTree, leaf, index);
      });

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );
      const onChainRoot =
        merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

      assert(
        Buffer.from(onChainRoot).equals(offChainTree.root),
        "Updated on chain root does not match root of updated off chain tree"
      );
    });
  });
});
