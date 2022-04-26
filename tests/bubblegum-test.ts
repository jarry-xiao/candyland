import * as anchor from "@project-serum/anchor";
import { BN, Provider, Program } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { Gummyroll } from "../target/types/gummyroll";
import {
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
  decodeMerkleRoll,
  getMerkleRollAccountSize,
} from "./merkle-roll-serde";
import { logTx } from "./utils";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

// @ts-ignore
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;

describe("bubblegum", () => {
  // Configure the client to use the local cluster.
  let offChainTree: Tree;
  let merkleRollKeypair: Keypair;

  const MAX_SIZE = 64;
  const MAX_DEPTH = 20;

  let payer = Keypair.generate();
  let connection = new web3Connection("http://localhost:8899", {
    commitment: "confirmed",
  });
  let wallet = new NodeWallet(payer);
  anchor.setProvider(
    new Provider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    })
  );
  Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;

  async function createTreeOnChain(payer: Keypair): Promise<[Keypair, Tree]> {
    const merkleRollKeypair = Keypair.generate();

    await Bubblegum.provider.connection.confirmTransaction(
      await Bubblegum.provider.connection.requestAirdrop(payer.publicKey, 1e10),
      "confirmed"
    );
    const requiredSpace = getMerkleRollAccountSize(MAX_DEPTH, MAX_SIZE);
    const leaves = Array(2 ** MAX_DEPTH).fill(Buffer.alloc(32));
    const tree = buildTree(leaves);

    const allocAccountIx = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: merkleRollKeypair.publicKey,
      lamports:
        await Bubblegum.provider.connection.getMinimumBalanceForRentExemption(
          requiredSpace
        ),
      space: requiredSpace,
      programId: GummyrollProgramId,
    });

    let [authority] = await PublicKey.findProgramAddress(
      [merkleRollKeypair.publicKey.toBuffer()],
      Bubblegum.programId
    );

    const initGummyrollIx = Bubblegum.instruction.createTree(
      MAX_DEPTH,
      MAX_SIZE,
      {
        accounts: {
          treeCreator: payer.publicKey,
          authority: authority,
          gummyrollProgram: GummyrollProgramId,
          merkleRoll: merkleRollKeypair.publicKey,
        },
        signers: [payer],
      }
    );

    let [nonce] = await PublicKey.findProgramAddress(
      [Buffer.from("bubblegum")],
      Bubblegum.programId
    );

    const initNonceIx = Bubblegum.instruction.initializeNonce({
      accounts: {
        nonce: nonce,
        payer: payer.publicKey,
        systemProgram: SystemProgram.programId,
      },
      signers: [payer],
    });

    const tx = new Transaction()
      .add(allocAccountIx)
      .add(initGummyrollIx)
      .add(initNonceIx);
    await Bubblegum.provider.send(tx, [payer, merkleRollKeypair], {
      commitment: "confirmed",
    });
    const merkleRoll = await Bubblegum.provider.connection.getAccountInfo(
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
      onChainMerkle.header.authority.equals(authority),
      "Failed to write auth pubkey"
    );

    assert(
      onChainMerkle.roll.changeLogs[0].root.equals(new PublicKey(tree.root)),
      "On chain root does not match root passed in instruction"
    );

    return [merkleRollKeypair, tree];
  }

  describe("Testing bubblgum", () => {
    beforeEach(async () => {
      let [computedMerkleRoll, computedOffChainTree] = await createTreeOnChain(
        payer
      );
      merkleRollKeypair = computedMerkleRoll;
      offChainTree = computedOffChainTree;
    });
    it("Mint to tree", async () => {});
  });
});
