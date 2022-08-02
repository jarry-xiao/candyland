import * as anchor from "@project-serum/anchor";
import { BN, AnchorProvider, Program } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";
import {
  buildTree,
  hash,
  getProofOfLeaf,
  updateTree,
  Tree,
} from "./merkle-tree";
import {
  createAllocTreeIx,
  getCreateTreeIxs,
  getMerkleRollAccountSize,
  createInitGummyrollWithRootInstruction,
  createInitGummyrollWithRootWithProofInstruction,
  createInitEmptyGummyrollInstruction,
  createAppendInstruction,
  createVerifyLeafInstruction,
  createVerifyLeafWithProofInstruction,
  createReplaceLeafInstruction,
  createReplaceLeafWithProofInstruction,
  createTransferAuthorityInstruction,
  decodeMerkleRoll
} from "@sorend-solana/gummyroll";
import {
  assertOnChainMerkleRollProperties
} from "./utils"
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { CANDY_WRAPPER_PROGRAM_ID, execute, logTx } from "@sorend-solana/utils";

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let merkleRollKeypair: Keypair;
  let payer: Keypair;
  let connection;
  let wallet;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 14;

  async function createTreeOnChain(
    payer: Keypair,
    numLeaves: number,
    maxDepth?: number,
    maxSize?: number,
    canopyDepth?: number
  ): Promise<[Keypair, Tree]> {
    if (maxDepth === undefined) {
      maxDepth = MAX_DEPTH;
    }
    if (maxSize === undefined) {
      maxSize = MAX_SIZE;
    }
    const merkleRollKeypair = Keypair.generate();

    const leaves = Array(2 ** maxDepth).fill(Buffer.alloc(32));
    for (let i = 0; i < numLeaves; i++) {
      leaves[i] = crypto.randomBytes(32);
    }
    const tree = buildTree(leaves);

    const allocAccountIx = await createAllocTreeIx(
      connection,
      maxSize,
      maxDepth,
      canopyDepth,
      payer.publicKey,
      merkleRollKeypair.publicKey
    );

    const ixs = [allocAccountIx];
    if (numLeaves > 0) {
      const root = Array.from(tree.root.map((x) => x));
      const leaf: number[] = Array.from(leaves[numLeaves - 1]);
      const proof = getProofOfLeaf(tree, numLeaves - 1).map((node) => {
        return node.node
      });

      let initGummyRollWithRootInstr = createInitGummyrollWithRootWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        },
        {
          maxDepth,
          maxBufferSize: maxSize,
          root,
          leaf,
          index: numLeaves-1,
          changelogDbUri: "https://arweave.net/<changelog_db_uri>",
          metadataDbUri: "https://arweave.net/<metadata_db_id>"
        },
        proof
      );
      ixs.push(
        initGummyRollWithRootInstr
      );
    } else {
      ixs.push(
        createInitEmptyGummyrollInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
          },
          {
            maxDepth,
            maxBufferSize: maxSize
          }
        )
      );
    }
    let txId = await execute(anchor.getProvider(), ixs, [
      payer,
      merkleRollKeypair,
    ]);
    if (canopyDepth) {
      await logTx(anchor.getProvider(), txId as string);
    }

    await assertOnChainMerkleRollProperties(
      connection,
      maxDepth,
      maxSize,
      payer.publicKey,
      new PublicKey(tree.root),
      merkleRollKeypair.publicKey
    );

    return [merkleRollKeypair, tree];
  }

  beforeEach(async () => {
    payer = Keypair.generate();
    connection = new web3Connection("http://localhost:8899", {
      commitment: "confirmed",
    });
    wallet = new NodeWallet(payer);
    anchor.setProvider(
      new AnchorProvider(connection, wallet, {
        commitment: connection.commitment,
        skipPreflight: true,
      })
    );

    await connection.confirmTransaction(
      await connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
  });

  describe("Having created a tree with a single leaf", () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(payer, 1);
    });
    it("Append single leaf", async () => {
      const newLeaf = crypto.randomBytes(32);
      const appendIx = createAppendInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
        },
        {
          leaf: Array.from(newLeaf)
        }
      )

      await execute(anchor.getProvider(), [appendIx], [payer]);

      updateTree(offChainTree, newLeaf, 1);

      const merkleRollAccount =
        await connection.getAccountInfo(
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
    it("Verify proof works for that leaf", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;
      const proof = getProofOfLeaf(offChainTree, index).map((treeNode) => {
        return treeNode.node;
      });

      let verifyLeafIx = createVerifyLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
        },
        {
          root: Array.from(offChainTree.root),
          leaf: Array.from(previousLeaf),
          index
        },
        proof
      )

      let replaceLeafIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(offChainTree.root),
          previousLeaf: Array.from(previousLeaf),
          newLeaf: Array.from(newLeaf),
          index
        },
        proof
      )

      await execute(anchor.getProvider(), [verifyLeafIx, replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await connection.getAccountInfo(
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
    it("Verify leaf fails when proof fails", async () => {
      const previousLeaf = offChainTree.leaves[0].node;
      const newLeaf = crypto.randomBytes(32);
      const index = 0;
      // Proof has random bytes: definitely wrong
      const proof = getProofOfLeaf(offChainTree, index).map((treeNode) => {
        return crypto.randomBytes(32);
      });

      let verifyLeafIx = createVerifyLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
        },
        {
          root: Array.from(offChainTree.root),
          leaf: Array.from(previousLeaf),
          index
        },
        proof
      )
      try {
        await execute(anchor.getProvider(), [verifyLeafIx], [payer]);
        assert(false, "Proof should have failed to verify");
      } catch {}

      // Replace instruction with same proof fails
      let replaceLeafIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(offChainTree.root),
          previousLeaf: Array.from(previousLeaf),
          newLeaf: Array.from(newLeaf),
          index
        },
        proof
      )
      try {
        await execute(anchor.getProvider(), [replaceLeafIx], [payer]);
        assert(false, "Replace should have failed to verify");
      } catch {}
      const merkleRollAccount =
        await connection.getAccountInfo(
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

      let replaceLeafIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(offChainTree.root),
          previousLeaf: Array.from(previousLeaf),
          newLeaf: Array.from(newLeaf),
          index
        },
        getProofOfLeaf(offChainTree, index, false, -1).map((treeNode) => {
          return treeNode.node;
        })
      )

      assert(
        replaceLeafIx.keys.length == 3 + MAX_DEPTH,
        `Failed to create proof for ${MAX_DEPTH}`
      );

      await execute(anchor.getProvider(), [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await connection.getAccountInfo(
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

      let replaceLeafIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(offChainTree.root),
          previousLeaf: Array.from(previousLeaf),
          newLeaf: Array.from(newLeaf),
          index
        },
        getProofOfLeaf(offChainTree, index, true, 1).map((treeNode) => {
          return treeNode.node;
        })
      )
      assert(
        replaceLeafIx.keys.length == 3 + 1,
        "Failed to minimize proof to expected size of 1"
      );
      await execute(anchor.getProvider(), [replaceLeafIx], [payer]);

      updateTree(offChainTree, newLeaf, index);

      const merkleRollAccount =
        await connection.getAccountInfo(
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

  describe("Examples tranferring appendAuthority", () => {
    const authority = Keypair.generate();
    const randomSigner = Keypair.generate();
    describe("Examples transferring authority", () => {
      it("... initializing tree ...", async () => {
        await connection.confirmTransaction(
          await (connection as Connection).requestAirdrop(
            authority.publicKey,
            1e10
          )
        );
        [merkleRollKeypair, offChainTree] = await createTreeOnChain(
          authority,
          1
        );
      });
      it("Can transfer authority", async () => {
        const transferAuthorityIx = createTransferAuthorityInstruction(
          {
            authority: authority.publicKey,
            merkleRoll: merkleRollKeypair.publicKey
          },
          {
            newAuthority: randomSigner.publicKey
          }
        )
        await execute(anchor.getProvider(), [transferAuthorityIx], [authority]);

        const merkleRoll = decodeMerkleRoll(
          (
            await connection.getAccountInfo(
              merkleRollKeypair.publicKey
            )
          ).data
        );
        const merkleRollInfo = merkleRoll.header;

        assert(
          merkleRollInfo.authority.equals(randomSigner.publicKey),
          `Upon transfering authority, authority should be ${randomSigner.publicKey.toString()}, but was instead updated to ${merkleRollInfo.authority.toString()}`
        );
      });
      it("Attempting to replace with new authority now works", async () => {
        const newLeaf = crypto.randomBytes(32);
        const replaceIndex = 0;
        const proof = getProofOfLeaf(offChainTree, replaceIndex);
        let replaceIx = createReplaceLeafWithProofInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: randomSigner.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            root: Array.from(offChainTree.root),
            previousLeaf: Array.from(offChainTree.leaves[replaceIndex].node),
            newLeaf: Array.from(newLeaf),
            index: replaceIndex
          },
          proof.map((treeNode) => {
            return treeNode.node;
          })
        )
        try {
          await execute(anchor.getProvider(), [replaceIx], [randomSigner]);
          assert(
            false,
            "Transaction should have failed since incorrect authority cannot execute replaces"
          );
        } catch {}
      });
    });
  });

  describe(`Having created a tree with ${MAX_SIZE} leaves`, () => {
    beforeEach(async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        MAX_SIZE
      );
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
        let replaceIx = createReplaceLeafWithProofInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            root: Array.from(offChainTree.root),
            previousLeaf: Array.from(offChainTree.leaves[i].node),
            newLeaf: Array.from(newLeaf),
            index
          },
          proof.map((treeNode) => {
            return treeNode.node;
          })
        )
        ixArray.push(replaceIx);
      }

      // Execute all replaces in a "single block"
      ixArray.map((ix) => {
        txList.push(
          execute(anchor.getProvider(), [ix], [payer])
        );
      });
      await Promise.all(txList);

      leavesToUpdate.map((leaf, index) => {
        updateTree(offChainTree, leaf, index);
      });

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await connection.getAccountInfo(
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
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        0,
        DEPTH,
        2 ** DEPTH
      );

      for (let i = 0; i < 2 ** DEPTH; i++) {
        const newLeaf = Array.from(Buffer.alloc(32, i + 1));
        const appendIx = createAppendInstruction(
          {
            authority: payer.publicKey,
            merkleRoll: merkleRollKeypair.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            leaf: newLeaf
          }
        )
        await execute(anchor.getProvider(), [appendIx], [payer]);
      }

      // Compare on-chain & off-chain roots
      const merkleRoll = decodeMerkleRoll(
        (
          await connection.getAccountInfo(
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
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
      let replaceIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(Buffer.alloc(32)),
          previousLeaf: Array.from(maliciousLeafHash),
          newLeaf: Array.from(maliciousLeafHash1),
          index: 0
        },
        nodeProof
      )

      try {
        await execute(anchor.getProvider(), [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) {}

      const merkleRoll = decodeMerkleRoll(
        (
          await connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.activeIndex === 0,
        "Merkle roll updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
    it("Random attacker fails to fake the existence of a leaf by autocompleting proof", async () => {
      const maliciousLeafHash = crypto.randomBytes(32);
      const maliciousLeafHash1 = crypto.randomBytes(32);
      const nodeProof = [];
      for (let i = 0; i < DEPTH; i++) {
        nodeProof.push(Buffer.alloc(32));
      }

      // Root - make this nonsense so it won't match what's in CL, and force proof autocompletion
      let replaceIx = createReplaceLeafWithProofInstruction(
        {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
          candyWrapper: CANDY_WRAPPER_PROGRAM_ID
        },
        {
          root: Array.from(Buffer.alloc(32)),
          previousLeaf: Array.from(maliciousLeafHash),
          newLeaf: Array.from(maliciousLeafHash1),
          index: 0
        },
        nodeProof
      )

      try {
        await execute(anchor.getProvider(), [replaceIx], [payer]);
        assert(
          false,
          "Attacker was able to succesfully write fake existence of a leaf"
        );
      } catch (e) {}

      const merkleRoll = decodeMerkleRoll(
        (
          await connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      assert(
        merkleRoll.roll.activeIndex === 0,
        "Merkle roll updated its active index after attacker's transaction, when it shouldn't have done anything"
      );
    });
  });
  describe(`Canopy test`, () => {
    const DEPTH = 5;
    it("Testing canopy for appends and replaces on a full on chain tree", async () => {
      [merkleRollKeypair, offChainTree] = await createTreeOnChain(
        payer,
        0,
        DEPTH,
        8,
        DEPTH // Store full tree on chain
      );

      let leaves = [];
      let i = 0;
      let stepSize = 4;
      while (i < 2 ** DEPTH) {
        let ixs = [];
        for (let j = 0; j < stepSize; ++j) {
          const newLeaf = Array.from(Buffer.alloc(32, i + 1));
          leaves.push(newLeaf);
          const appendIx = createAppendInstruction(
            {
              authority: payer.publicKey,
              merkleRoll: merkleRollKeypair.publicKey,
              candyWrapper: CANDY_WRAPPER_PROGRAM_ID
            },
            {
              leaf: Array.from(newLeaf)
            }
          )
          ixs.push(appendIx);
        }
        await execute(anchor.getProvider(), ixs, [payer]);
        i += stepSize;
        console.log("Appended", i, "leaves");
      }

      // Compare on-chain & off-chain roots
      let ixs = [];
      const merkleRoll = decodeMerkleRoll(
        (
          await connection.getAccountInfo(
            merkleRollKeypair.publicKey
          )
        ).data
      );

      let root = merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root;
      let leafList = Array.from(leaves.entries());
      leafList.sort(() => Math.random() - 0.5);
      let replaces = 0;
      let newLeaves = {};
      for (const [i, leaf] of leafList) {
        const newLeaf = crypto.randomBytes(32);
        newLeaves[i] = newLeaf;
        const replaceIx = createReplaceLeafInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            root: Array.from(root.toBuffer()),
            previousLeaf: Array.from(leaf),
            newLeaf: Array.from(newLeaf),
            index: i
          }
        )
        ixs.push(replaceIx);
        if (ixs.length == stepSize) {
          replaces++;
          await execute(anchor.getProvider(), ixs, [payer]);
          console.log("Replaced", replaces * stepSize, "leaves");
          ixs = [];
        }
      }

      let newLeafList = []
      for (let i = 0; i < 32; ++i)  {
        newLeafList.push(newLeaves[i])
      }

      let tree = buildTree(newLeafList)


      for (let proofSize = 1; proofSize <= 5; ++proofSize) {
        const newLeaf = crypto.randomBytes(32);
        let i = Math.floor(Math.random() * 32)
        const leaf = newLeaves[i];
        
        let partialProof = getProofOfLeaf(tree, i).slice(0, proofSize).map((n) => n.node)
        console.log(`Replacing node ${i}, proof length = ${proofSize}`)
        for (const [level, node] of Object.entries(partialProof)) {
          console.log(` ${level}: ${bs58.encode(node)}`)
        }

        let replaceIx = createReplaceLeafWithProofInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            root: Array.from(root.toBuffer()),
            previousLeaf: Array.from(newLeaves[i]),
            newLeaf: Array.from(newLeaf),
            index: i
          },
          partialProof
        )

        updateTree(tree, newLeaf, i);
        let replaceBackIx = createReplaceLeafWithProofInstruction(
          {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
            candyWrapper: CANDY_WRAPPER_PROGRAM_ID
          },
          {
            root: Array.from(tree.root),
            previousLeaf: Array.from(newLeaf),
            newLeaf: Array.from(newLeaves[i]),
            index: i
          },
          partialProof
        )
        updateTree(tree, leaf, i);
        await execute(anchor.getProvider(), [replaceIx, replaceBackIx], [payer], true, true);
      }
    });
  });
});
