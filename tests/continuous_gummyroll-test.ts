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

  function createReplaceId(tree: Tree, i: number) {
    let newLeaf = Buffer.alloc(32, Buffer.from(Uint8Array.from([i])));
    let nodeProof = getProofOfLeaf(tree, i).map((node) => { return { pubkey: new PublicKey(node.node), isSigner: false, isWritable: false } });
    const replaceLeafIx = program.instruction.replaceLeaf(
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

  it("Continuous updating and syncing", async () => {
    let indicesToSend = [];
    for (let i = 0; i < 500; i++) {
        indicesToSend.push(i);
    };

    while (indicesToSend.length > 0) {
        const txIds = [];
        for (const i of indicesToSend) {
            txIds.push(
                sendAndConfirmTransaction(
                    connection,
                    new Transaction().add(createReplaceId(tree, i)),
                    [payer],
                    {
                        commitment: 'confirmed',
                        skipPreflight: true,
                        
                    }
                )
                    .then((_txid) => null)
                    .catch((reason) => { 
                        // console.error(reason);
                        return i
                    })
            );
        }

        indicesToSend = (await Promise.all(txIds)).filter((err) => !!err);
        console.log(`${indicesToSend.length} txs failed!`);
    }

    const okToCompare = await sleep(5000);
    console.log("Okay to compare:", okToCompare);
    console.log("Events processed:", eventsProcessed.get('0'));

    const merkleRoll = await program.provider.connection.getAccountInfo(
        merkleRollKeypair.publicKey
    );
    let onChainMerkle = decodeMerkleRoll(merkleRoll.data);
    
    const onChainRoot = onChainMerkle.roll.changeLogs[onChainMerkle.roll.activeIndex].root;
    const treeRoot = new PublicKey(tree.root);
    console.log("onChainRoot:", onChainRoot.toString());
    console.log("offChainRoot:", treeRoot.toString());
    console.log("Active index:", onChainMerkle.roll.activeIndex);
    console.log("Buffer size:", onChainMerkle.roll.bufferSize);

    assert(
        onChainRoot.equals(treeRoot),
        "On chain root does not match root passed in instruction"
    );
  });

  it("Kill listeners", async () => {
    await program.removeEventListener(listener);
  });
});
