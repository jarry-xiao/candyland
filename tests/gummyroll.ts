
import * as anchor from "@project-serum/anchor";
import { Gummyroll } from "../target/types/gummyroll" ;
import { Program, BN } from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import {
  DataV2,
  CreateMetadataV2,
  MetadataProgram,
  CreateMasterEditionV3,
  MasterEditionV2Data,
} from "@metaplex-foundation/mpl-token-metadata";
import { PublicKey, Keypair, SystemProgram, Transaction } from "@solana/web3.js";
import { Token, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { assert } from "chai";

import { MAX_SIZE, buildTree, hash, getProofOfLeaf, updateTree, hashLeaves } from './merkle-tree';

const TOKEN_PROGRAM_2022_ID = new PublicKey(
  "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
);

const logTx = async (provider, tx) => {
  await provider.connection.confirmTransaction(tx, "confirmed");
  console.log(
    (await provider.connection.getConfirmedTransaction(tx, "confirmed")).meta
      .logMessages
  );
};

async function checkTxStatus(provider: anchor.Provider, tx: string): Promise<boolean> {
  let metaTx = await provider.connection.getTransaction(tx, { commitment: "confirmed" });
  return metaTx.meta.err === null;
}

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Gummyroll as Program<Gummyroll>;
  const payer = Keypair.generate();

  const merkleRollKeypair = Keypair.generate();
  console.log("Payer key:", payer.publicKey);

  const requiredSpace = 44248 + 8;
  const leaves = Array(2**20).fill(Buffer.alloc(32));
  leaves[0] = Keypair.generate().publicKey.toBuffer();
  let tree = buildTree(leaves);
  console.log("Created root using leaf pubkey: ", leaves[0]);
  console.log("program id:", program.programId.toString());

  it("Initialize keypairs with Sol", async () => {
    await program.provider.connection.confirmTransaction(
        await program.provider.connection.requestAirdrop(payer.publicKey, 1e10),
        "confirmed"
    )
    await program.provider.connection.confirmTransaction(
        await program.provider.connection.requestAirdrop(payer.publicKey, 1e10),
        "confirmed"
    )
  });

  it("Initialize root with prepopulated leaves", async () => {
    const allocAccountIx = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: merkleRollKeypair.publicKey,
        lamports: await program.provider.connection.getMinimumBalanceForRentExemption(requiredSpace),
        space: requiredSpace,
        programId: program.programId,
    });

    const root = { inner: Array.from(tree.root) };
    const leaf = { inner: Array.from(leaves[0]) };
    const proof = getProofOfLeaf(tree, 0).map((node) => { return {inner : Array.from(node.node) }});
    const initGummyrollIx = await program.instruction.initGummyrollWithRoot(
        root, leaf, proof, 0,
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                authority: payer.publicKey,
            },
            signers: [payer],
        }
    );

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await program.provider.send(tx, [payer, merkleRollKeypair], {
        commitment: 'confirmed'
    })
    await logTx(program.provider, txid);
    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    assert(
        Buffer.from(merkleRoll.roots[0].inner).equals(tree.root),
        "On chain root matches root passed in instruction", 
    );
  });
  it("Append single leaf", async () => {
    const newLeaf = hash(payer.publicKey.toBuffer(), payer.publicKey.toBuffer());

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
        commitment: 'confirmed',
    });
    logTx(program.provider, txid);

    updateTree(tree, newLeaf, 1, true);

    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    const onChainRoot = merkleRoll.roots[merkleRoll.activeIndex.toNumber()].inner;
    console.log(Uint8Array.from(onChainRoot), Uint8Array.from(tree.root));

    assert(
        Buffer.from(onChainRoot).equals(tree.root),
        "Updated on chain root matches root of updated off chain tree", 
    );
  });
  it("Replace single leaf", async () => {
    const previousLeaf = Buffer.alloc(32);
    const newLeaf = hash(payer.publicKey.toBuffer(), payer.publicKey.toBuffer());
    const index = 2;
    const proof = getProofOfLeaf(tree, index);

    const nodeProof = proof.map((treeNode) => { return { inner: treeNode.node }});

    const replaceLeafIx = await program.instruction.replaceLeaf(
        { inner: Array.from(tree.root) },
        { inner: Array.from(previousLeaf) },
        { inner: Array.from(newLeaf) },
        nodeProof,
        index,
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                authority: payer.publicKey,
            },
            signers: [payer],
        }
    );

    const tx = new Transaction().add(replaceLeafIx);
    const txid = await program.provider.send(tx, [payer], {
        commitment: 'confirmed',
    });
    logTx(program.provider, txid);

    updateTree(tree, newLeaf, index, true);

    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    const onChainRoot = merkleRoll.roots[merkleRoll.activeIndex.toNumber()].inner;
    console.log(Uint8Array.from(onChainRoot), Uint8Array.from(tree.root));

    assert(
        Buffer.from(onChainRoot).equals(tree.root),
        "Updated on chain root matches root of updated off chain tree", 
    );
  });
  it.skip("Replace leaf - max block (64)", async () => {
    /// Replace 64 leaves before syncing off-chain tree with on-chain tree

    let changeArray = [];
    let txList = [];

    for(let i = 0; i < MAX_SIZE; i++) {
        const index = 3+i;
        const newLeaf = hash(payer.publicKey.toBuffer(), Buffer.from(new BN(i).toArray()));
        const proof = getProofOfLeaf(tree, index);

        /// Use this to sync off-chain tree
        changeArray.push({newLeaf, index});

        const nodeProof = proof.map((treeNode) => { return { inner: treeNode.node } });

        const replaceLeafIx = await program.instruction.replaceLeaf(
            { inner: Array.from(tree.root) },
            { inner: Array.from(Buffer.alloc(32)) },
            { inner: Array.from(newLeaf) },
            nodeProof,
            index,
            {
                accounts: {
                    merkleRoll: merkleRollKeypair.publicKey,
                    authority: payer.publicKey,
                },
                signers: [payer],
            }
        );

        const tx = new Transaction().add(replaceLeafIx);
        txList.push(
            program.provider.send(tx, [payer], {
                commitment: 'confirmed',
            })
            .then((txId) => checkTxStatus(program.provider, txId))
            .then((txOk) => { if (!txOk) { throw Error("Encountered failed tx")} })
        );
    }
    await Promise.all(txList);

    changeArray.forEach((change) => {
        updateTree(tree, change.newLeaf, change.index);
    })
    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    const onChainRoot = merkleRoll.roots[merkleRoll.activeIndex.toNumber()].inner;
    assert(
        Buffer.from(onChainRoot).equals(tree.root),
        "Updated on chain root matches root of updated off chain tree", 
    );
  });
  it.skip("Replace leaf - max block + 1 (65)", async () => {
    /// Replace more leaves than MAX_SIZE, which should fail

    let changeArray = [];
    let txList = [];

    const offset = 3+MAX_SIZE;
    for(let i = 0; i < MAX_SIZE+1; i++) {
        const index = offset+i;
        const newLeaf = hash(payer.publicKey.toBuffer(), Buffer.from(new BN(i).toArray()));
        const proof = getProofOfLeaf(tree, index);

        /// Use this to sync off-chain tree
        changeArray.push({newLeaf, index});

        const nodeProof = proof.map((treeNode) => { return { inner: treeNode.node } });

        const replaceLeafIx = await program.instruction.replaceLeaf(
            { inner: Array.from(tree.root) },
            { inner: Array.from(Buffer.alloc(32)) },
            { inner: Array.from(newLeaf) },
            nodeProof,
            index,
            {
                accounts: {
                    merkleRoll: merkleRollKeypair.publicKey,
                    authority: payer.publicKey,
                },
                signers: [payer],
            }
        );

        const tx = new Transaction().add(replaceLeafIx);
        txList.push(
            program.provider.send(tx, [payer], {
                commitment: 'confirmed',
            })
            .then((txId) => checkTxStatus(program.provider, txId))
            .catch(() => {return false})
        );
    }
    let txIds = await Promise.all(txList)
    let failures = txIds.map((txOk) => Number(!txOk)).reduce(
        (left, right) => left + right,
        0
    );
    assert(failures === 1, "Exactly 1 failure");
  });
});
