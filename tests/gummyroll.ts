
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

import { buildTree, hash, getProofOfLeaf, updateTree } from './merkle-tree';

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

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Gummyroll as Program<Gummyroll>;
  const payer = Keypair.generate();

  const merkleRollKeypair = Keypair.generate();
  console.log("Payer key:", payer.publicKey);

  const requiredSpace = 43568 + 8;
  const leaves = Array(2**20).fill(Buffer.alloc(32));
  leaves[0] = Keypair.generate().publicKey.toBuffer();
  let tree = buildTree(leaves);
  console.log("Created root using leaf pubkey: ", tree.root);
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

    const initGummyrollIx = await program.instruction.initGummyroll(
        { inner: Array.from(tree.root) },
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                payer: payer.publicKey,
                systemProgram: anchor.web3.SystemProgram.programId,
            },
            signers: [payer],
        }
    );
    // console.log("init gummy roll ix:", initGummyrollIx);

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await program.provider.send(tx, [payer, merkleRollKeypair], {
        commitment: 'confirmed'
    })
    await logTx(program.provider, txid);
    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    // console.log("Loaded merkleRoll...");
    // console.log("            root:", Buffer.from(merkleRoll.roots[0].inner));
    // console.log("       expecting:", tree.root);
    assert(
        "On chain root matches root passed in instruction", 
        Buffer.from(merkleRoll.roots[0].inner) === tree.root
    );
  });
  it("Replace leaf", async () => {
    const previousLeaf = Buffer.alloc(32);
    const newLeaf = hash(payer.publicKey.toBuffer(), payer.publicKey.toBuffer());
    const index = 1;
    const [proof, path] = getProofOfLeaf(tree, index);
    // console.log("typescript path: ", path);
    // console.log("proof:", proof);

    const nodeProof = proof.map((treeNode) => { return { inner: treeNode.node }});
    // console.log("nodeProof:", nodeProof);

    updateTree(tree, newLeaf, index);

    const replaceLeafIx = await program.instruction.replaceLeaf(
        { inner: Array.from(tree.root) },
        { inner: Array.from(previousLeaf) },
        { inner: Array.from(newLeaf) },
        nodeProof,
        index,
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                payer: payer.publicKey,
            },
            signers: [payer],
        }
    );
    // console.log("Replace leaf ix:", replaceLeafIx);

    const tx = new Transaction().add(replaceLeafIx);
    const txid = await program.provider.send(tx, [payer], {
        commitment: 'confirmed',
    });
    logTx(program.provider, txid);

    const merkleRoll = await program.account.merkleRoll.fetch(merkleRollKeypair.publicKey);
    const onChainRoot = merkleRoll.roots[merkleRoll.activeIndex.toNumber()].inner;
    // console.log("Merkle roll loaded...");
    // console.log("       root:", onChainRoot);
    // console.log("   expected:", tree.root);

    assert(
        "Updated on chain root matches root of updated off chain tree", 
        Buffer.from(onChainRoot) === tree.root
    );
  });
});