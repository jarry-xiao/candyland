import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll";
import {Program, BN, Provider} from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction, Connection,
} from "@solana/web3.js";
import * as borsh from 'borsh';
import { assert } from "chai";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { buildTree, hash, getProofOfLeaf, updateTree } from "./merkle-tree";
import { decodeMerkleRoll, getMerkleRollAccountSize, OnChainMerkleRoll } from "./merkle-roll-serde";

const logTx = async (provider, tx) => {
  await provider.connection.confirmTransaction(tx, "confirmed");
  console.log(
    (await provider.connection.getConfirmedTransaction(tx, "confirmed")).meta
      .logMessages
  );
};

async function checkTxStatus(
  provider: anchor.Provider,
  tx: string,
  verbose = false
): Promise<boolean> {
  if (verbose) {
    await logTx(provider, tx);
  }
  let metaTx = await provider.connection.getTransaction(tx, {
    commitment: "confirmed",
  });
  return metaTx.meta.err === null;
}

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  const connection = new Connection(
      "http://localhost:8899",
      {
        commitment: 'confirmed'
      }
  );
  const payer = Keypair.generate();
  const accnt = new NodeWallet(payer)
  anchor.setProvider(new Provider(connection, accnt, {commitment: connection.commitment, skipPreflight: true} ));

  /// @ts-ignore
  const program = anchor.workspace.Gummyroll as Program<Gummyroll>;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 22;

  const merkleRollKeypair = Keypair.generate();
  console.log("Payer key:", payer.publicKey);

  const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
  const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
  leaves[0] = Keypair.generate().publicKey.toBuffer();
  let tree = buildTree(leaves);
  console.log("Created root using leaf pubkey: ", Uint8Array.from(leaves[0]));
  console.log("program id:", program.programId.toString());
  let listener = program.addEventListener("ChangeLogEvent", (event) => {
    updateTree(tree, Buffer.from(event.path[0].inner), event.index);
  });

  it("Initialize keypairs with Sol", async () => {
    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
  });

  it("Initialize root with prepopulated leaves", async () => {
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

    const root = { inner: Array.from(tree.root) };
    const leaf = { inner: Array.from(leaves[0]) };
    const proof = getProofOfLeaf(tree, 0).map((node) => {
      return { pubkey: new PublicKey(node.node), isSigner: false, isWritable: false };
    });
    const initGummyrollIx = await program.instruction.initGummyrollWithRoot(
      MAX_DEPTH,
      MAX_SIZE,
      root,
      leaf,
      0,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: proof
      }
    );

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await program.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
    });
    await logTx(program.provider, txid);
    const merkleRoll = await program.provider.connection.getAccountInfo(
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
  it("Append single leaf", async () => {
    const newLeaf = hash(
      payer.publicKey.toBuffer(),
      payer.publicKey.toBuffer()
    );

    const appendIx = await program.instruction.append(
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
    const txid = await program.provider.send(tx, [payer], {
      commitment: "confirmed",
    });
    await logTx(program.provider, txid);

    updateTree(tree, newLeaf, 1);

    const merkleRollAccount = await program.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );
    const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
    const onChainRoot =
      merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

    assert(
      Buffer.from(onChainRoot).equals(tree.root),
      "Updated on chain root matches root of updated off chain tree"
    );
  });
  it("Replace single leaf", async () => {
    const previousLeaf = Buffer.alloc(32);
    const newLeaf = hash(
      payer.publicKey.toBuffer(),
      payer.publicKey.toBuffer()
    );
    const index = 2;
    const proof = getProofOfLeaf(tree, index);

    const nodeProof = proof.map((treeNode) => {
      return { pubkey: new PublicKey(treeNode.node), isSigner: false, isWritable: false };
    });

    const replaceLeafIx = program.instruction.replaceLeaf(
      { inner: Array.from(tree.root) },
      { inner: Array.from(previousLeaf) },
      { inner: Array.from(newLeaf) },
      index,
      {
        accounts: {
          merkleRoll: merkleRollKeypair.publicKey,
          authority: payer.publicKey,
        },
        signers: [payer],
        remainingAccounts: nodeProof
      }
    );

    const tx = new Transaction().add(replaceLeafIx);
    const txid = await program.provider.send(tx, [payer], {
      commitment: "confirmed",
    });
    await logTx(program.provider, txid);

    updateTree(tree, newLeaf, index);

    const merkleRollAccount = await program.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );
    const merkleRoll = decodeMerkleRoll(merkleRollAccount.data);
    const onChainRoot =
      merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

    assert(
      Buffer.from(onChainRoot).equals(tree.root),
      "Updated on chain root matches root of updated off chain tree"
    );
  });
  it(`Replace leaf - max block ${MAX_SIZE}`, async () => {
    /// Replace 64 leaves before syncing off-chain tree with on-chain tree

    let changeArray = [];
    let txList = [];

    const failedRoot = { inner: Array.from(tree.root) };
    const failedLeaf = { inner: Array.from(tree.leaves[2].node) };
    const failedProof = getProofOfLeaf(tree, 2).map(
        (treeNode) => {
            return { pubkey: new PublicKey(treeNode.node), isSigner: false, isWritable: false };
        }
    )

    for (let i = 0; i < MAX_SIZE; i++) {
      const index = 3 + i;
      const newLeaf = hash(
        payer.publicKey.toBuffer(),
        Buffer.from(new BN(i).toArray())
      );
      const proof = getProofOfLeaf(tree, index);

      /// Use this to sync off-chain tree
      changeArray.push({ newLeaf, index });

      const nodeProof = proof.map((treeNode) => {
        return { pubkey: new PublicKey(treeNode.node), isSigner: false, isWritable: false };
      });

      const insertOrAppendIx = await program.instruction.insertOrAppend(
        { inner: Array.from(tree.root) },
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

      const tx = new Transaction().add(insertOrAppendIx);
      txList.push(
        program.provider.send(tx, [payer], {
          commitment: "confirmed",
          skipPreflight: true,
        })
      );
    }
    await Promise.all(txList);

    

    // Compare on-chain & off-chain roots
    const merkleRoll = decodeMerkleRoll((await program.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    )).data);
    const onChainRoot =
      merkleRoll.roll.changeLogs[merkleRoll.roll.activeIndex].root.toBuffer();

    assert(
      Buffer.from(onChainRoot).equals(tree.root),
      "Updated on chain root does not match root of updated off chain tree"
    );

    try {
      await program.rpc.replaceLeaf(
        failedRoot,
        failedLeaf,
        Buffer.alloc(32),
        2,
        {
          accounts: {
            merkleRoll: merkleRollKeypair.publicKey,
            authority: payer.publicKey,
          },
          signers: [payer],
          remainingAccounts: failedProof,
        }
      );
      console.log("Unexpected success");
      assert(false);
    } catch (e) {
      console.log("Expected failure");
    }
  });

  it("Kill listeners", async () => {
    await program.removeEventListener(listener);
  });
});
