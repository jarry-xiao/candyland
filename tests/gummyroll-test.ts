import * as anchor from "@project-serum/anchor";
import {BN, TransactionNamespace, InstructionNamespace, Provider, Program} from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  TransactionInstruction, Connection as web3Connection
} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from 'crypto';

import { buildTree, hash, getProofOfLeaf, updateTree, Tree } from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
} from "./merkle-roll-serde";
import { logTx } from "./utils";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

// @ts-ignore
let Gummyroll;

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;
  let connection;
  let wallet;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;

  async function createTreeOnChain(
    payer: Keypair,
    numLeaves: number,
    maxDepth?: number,
    maxSize?: number,
  ): Promise<[Keypair, Tree]> {
    if (maxDepth === undefined) { maxDepth = MAX_DEPTH }
    if (maxSize === undefined) { maxSize = MAX_SIZE }
    const merkleRollKeypair = Keypair.generate();

    const requiredSpace = getMerkleRollAccountSize(maxDepth, maxSize);
    const leaves = Array(2 ** maxDepth).fill(Buffer.alloc(32));
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

    let tx = new Transaction().add(allocAccountIx);
    if (numLeaves > 0) {

      const root = { inner: Array.from(tree.root) };
      const leaf = { inner: Array.from(leaves[numLeaves - 1]) };
      const proof = getProofOfLeaf(tree, numLeaves - 1).map((node) => {
        return {
          pubkey: new PublicKey(node.node),
          isSigner: false,
          isWritable: false,
        };
      });

      tx = tx.add(Gummyroll.instruction.initGummyrollWithRoot(
        maxDepth,
        maxSize,
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
      ));
    } else {
      tx = tx.add(Gummyroll.instruction.initEmptyGummyroll(
        maxDepth,
        maxSize,
        {
          accounts: {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
          },
          signers: [payer],
        }
      ));
    }

    await Gummyroll.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
    });
    const merkleRoll = await Gummyroll.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );

    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    // Check header bytes are set correctly
    assert(
      onChainMerkle.header.maxDepth === maxDepth,
      `Max depth does not match ${onChainMerkle.header.maxDepth}, expected ${maxDepth}`
    );
    assert(
      onChainMerkle.header.maxBufferSize === maxSize,
      `Max buffer size does not match ${onChainMerkle.header.maxBufferSize}, expected ${maxSize}`
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
    connection = new web3Connection(
        "http://localhost:8899",
        {
          commitment: 'confirmed'
        }
    );
    wallet = new NodeWallet(payer)
    anchor.setProvider(new Provider(connection, wallet, { commitment: connection.commitment, skipPreflight: true }));
    Gummyroll = anchor.workspace.Gummyroll as Program<Gummyroll>;

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

  describe(`Having created a tree with depth 3`, () => {
    const DEPTH = 3;
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(payer, 0, DEPTH, 2 ** DEPTH);

      for (let i = 0; i < 2 ** DEPTH; i++) {
        const appendIx = Gummyroll.instruction.append(
          {
            inner: Array.from(Buffer.alloc(32, i + 1))
          },
          {
            accounts: {
              merkleRoll: merkleRollKeypair.publicKey,
              authority: payer.publicKey,
            },
            signers: [payer],
          }
        );
        const tx = new Transaction().add(appendIx);
        await Gummyroll.provider.send(tx, [payer], {
          commitment: "confirmed",
        });
      }

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.bufferSize === 2 ** DEPTH,
        "Not all changes were processed"
      );
      assert(
        merkleRoll.roll.activeIndex === 0,
        "Not all changes were processed"
      );
    });

    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push({ pubkey: new PublicKey(Buffer.alloc(32)), isSigner: false, isWritable: false });
      }

      const replaceIx = Gummyroll.instruction.replaceLeaf(
        // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
        { inner: Buffer.alloc(32) },
        { inner: maliciousLeafHash },
        { inner: maliciousLeafHash1 },
        0,
        {
          accounts: {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
          },
          signers: [payer],
          remainingAccounts: nodeProof,
        }
      );

      const tx = new Transaction().add(replaceIx);
      try {
        await Gummyroll.provider.send(tx, [payer], { commitment: "confirmed" });
        assert(false, "Attacker was able to succesfully write fake existence of a leaf");
      } catch (e) {

      }

      const merkleRoll = decodeMerkleRoll(
        (
          await Gummyroll.provider.connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.activeIndex === 0,
        "Merkle roll updated its active index after attacker's transaction, when it shouldn't have done anything"
      )
    });
  });
});
