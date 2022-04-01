import * as anchor from "@project-serum/anchor";
import * as crypto from 'crypto';
import { Gummyroll } from "../target/types/gummyroll";
import { Program, BN, Provider } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import { assert } from "chai";

import { buildTree, hash, getProofOfLeaf, updateTree } from "./merkle-tree";
import { decodeMerkleRoll, getMerkleRollAccountSize, OnChainMerkleRoll } from "./merkle-roll-serde";
import { logTx } from './utils';
import { sleep } from "../deps/metaplex-program-library/metaplex/js/test/utils";


const MAX_SIZE = 1024;
const MAX_DEPTH = 22;

describe("gummyroll-continuous", () => {
  const connection = new Connection(
      "http://localhost:8899",
      {
          commitment: 'confirmed'
      }
  );
  console.log(connection);
  const payer = Keypair.generate();
  const wallet = new NodeWallet(payer)
  anchor.setProvider(new Provider(connection, wallet, {commitment: connection.commitment, skipPreflight: true} ));
  
  /// @ts-ignore
  const program = anchor.workspace.Gummyroll as Program<Gummyroll>;


  const merkleRollKeypair = Keypair.generate();
  console.log("Payer key:", payer.publicKey);

  const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
  
  const leaves = Array(2 ** MAX_DEPTH).fill(crypto.randomBytes(32));
  let tree = buildTree(leaves);
  console.log("program id:", program.programId.toString());

  let eventsProcessed = new Map<String, number>();
  eventsProcessed.set("0", 0);

  let listener = program.addEventListener("ChangeLogEvent", (event) => {
    updateTree(tree, Buffer.from(event.path[0].inner), event.index);
    eventsProcessed.set("0", eventsProcessed.get("0") + 1);
  });

  it("Initialize keypairs with Sol", async () => {
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

    const initGummyrollIx = program.instruction.initGummyrollWithRoot(
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




  it("Continuous updating and syncing", async () => {
    let txs = [];
    for (let i = 0; i < 1000000; i++) {
        let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([i])));
        let nodeProof = getProofOfLeaf(tree, i).map((node) => { return { pubkey: new PublicKey(node.node), isSigner: false, isWritable: false } });
        const replaceLeaf = program.instruction.replaceLeaf(
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
        if (i % 100 == 0) { console.log("Sent ith tx:", i); }

        const tx = new Transaction().add(replaceLeaf);

        tx.feePayer = payer.publicKey;
        tx.recentBlockhash = (
            await connection.getLatestBlockhash('confirmed')
        ).blockhash;

        await wallet.signTransaction(tx);
        const rawTx = tx.serialize();

        txs.push(
            connection.sendRawTransaction(rawTx, { skipPreflight: true })
                .then((_txid) => { return true })
                .catch((reason) => { 
                    console.error(reason);
                    return false 
                })
        );

        //await sleep(100);
        await sleep(10);
    }
    let transactions = await Promise.all(txs);
    console.log("Txs:", transactions)

    let numSuccess = transactions.reduce((left, right) => {return left + Number(right)}, 0);
    console.log(`${numSuccess} txs succeeded!`);

    const merkleRoll = await program.provider.connection.getAccountInfo(
      merkleRollKeypair.publicKey
    );
    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);

    console.log("Num events processed: ", eventsProcessed.get('0'));
    sleep(2000);
    console.log("Num events processed: ", eventsProcessed.get('0'));

    assert(
      onChainMerkle.roll.changeLogs[onChainMerkle.roll.activeIndex].root.equals(new PublicKey(tree.root)),
      "On chain root does not match root passed in instruction"
    );
  });

  it("Kill listeners", async () => {
    await program.removeEventListener(listener);
  });
});
