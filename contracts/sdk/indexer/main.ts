import { Program, web3 } from "@project-serum/anchor";
import { bootstrap, NFTDatabaseConnection, Proof } from "./db";
import {
  createAppendIx,
  createReplaceIx,
  getMerkleRollAccountSize,
  Gummyroll,
} from "../gummyroll";
import * as crypto from "crypto";
import * as anchor from "@project-serum/anchor";
import { Keypair, SystemProgram, Transaction } from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  getUpdatedBatch,
  updateMerkleRollLive,
  updateMerkleRollSnapshot,
} from "./indexerGummyroll";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import {
  buildTree,
  getProofOfLeaf,
  Tree,
  TreeNode,
  updateTree,
} from "../../tests/merkle-tree";

async function sendAppendTransaction(
  GummyrollCtx: any,
  payer: any,
  merkleRollKeypair: any
) {
  const newLeaf = crypto.randomBytes(32);
  const appendTx = new Transaction().add(
    createAppendIx(
      GummyrollCtx,
      newLeaf,
      payer,
      payer,
      merkleRollKeypair.publicKey
    )
  );
  await GummyrollCtx.provider.send(appendTx, [payer], {
    commitment: "confirmed",
  });
  return newLeaf;
}

async function main() {
  const connection = new web3.Connection("http://127.0.0.1:8899", {
    commitment: "confirmed",
  });
  const payer = Keypair.generate();
  const wallet = new NodeWallet(payer);
  anchor.setProvider(
    new anchor.Provider(connection, wallet, {
      commitment: connection.commitment,
      skipPreflight: true,
    })
  );
  let GummyrollCtx = anchor.workspace.Gummyroll as Program<Gummyroll>;
  await GummyrollCtx.provider.connection.confirmTransaction(
    await GummyrollCtx.provider.connection.requestAirdrop(
      payer.publicKey,
      1e10
    ),
    "confirmed"
  );

  let maxDepth = 20;
  let maxSize = 1024;
  const merkleRollKeypair = Keypair.generate();

  const requiredSpace = getMerkleRollAccountSize(maxDepth, maxSize);

  const allocAccountIx = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    newAccountPubkey: merkleRollKeypair.publicKey,
    lamports:
      await GummyrollCtx.provider.connection.getMinimumBalanceForRentExemption(
        requiredSpace
      ),
    space: requiredSpace,
    programId: GummyrollCtx.programId,
  });

  let tx = new Transaction().add(allocAccountIx);
  tx = tx.add(
    GummyrollCtx.instruction.initEmptyGummyroll(maxDepth, maxSize, {
      accounts: {
        merkleRoll: merkleRollKeypair.publicKey,
        authority: payer.publicKey,
        appendAuthority: payer.publicKey,
      },
      signers: [payer],
    })
  );
  let initTx = await GummyrollCtx.provider.send(
    tx,
    [payer, merkleRollKeypair],
    {
      commitment: "confirmed",
    }
  );
  console.log(initTx);
  let nftDb = await bootstrap();
  console.log("Finished bootstrapping DB");
  await updateMerkleRollSnapshot(
    connection,
    merkleRollKeypair.publicKey,
    async (merkleRoll) => await getUpdatedBatch(merkleRoll, nftDb)
  );
  let subId = await updateMerkleRollLive(
    connection,
    merkleRollKeypair.publicKey,
    async (merkleRoll) => await getUpdatedBatch(merkleRoll, nftDb)
  );

  // TODO simulate a candy machine mint + ownership transfers

  let appends = 0;
  let replaces = 0;
  let failed = 0;

  let counter = 0;
  let replacedInds = [];
  let emptyLeaves: Buffer[] = [];
  for (let i = 0; i < 1 << 20; ++i) {
    emptyLeaves.push(nftDb.emptyNode(0));
  }
  let offChainMerkle = buildTree(emptyLeaves);

  while (1) {
    counter += 1;
    if (counter % 10 == 0) {
      console.log("Status: ", counter);
      await logStats(
        nftDb,
        offChainMerkle,
        appends,
        replaces,
        failed,
        replacedInds
      );
    }
    if (Math.random() < 0.5) {
      let leaf = await sendAppendTransaction(
        GummyrollCtx,
        payer,
        merkleRollKeypair
      );
      console.log(`Append ${bs58.encode(leaf)}, index: ${appends}`);
      updateTree(offChainMerkle, leaf, appends);
      appends += 1;
    } else {
      continue;
      let proof;
      let leaves;
      let leaf;
      let sample;
      leaves = await nftDb.getAllLeaves();
      if (leaves.size === 0) {
        console.log("No leaves in DB");
        continue;
      }
      sample = Math.floor(Math.random() * leaves.size);
      leaf = Array.from(leaves)[sample];
      if (!leaf) {
        continue;
      }
      proof = await nftDb.getProof(bs58.decode(leaf));
      if (!proof) {
        await logStats(
          nftDb,
          offChainMerkle,
          appends,
          replaces,
          failed,
          replacedInds
        );
        continue;
      }
      console.log(`Sampled ${bs58.encode(proof.leaf)}, index ${sample}`);
      let newLeaf = crypto.randomBytes(32);
      replaces += 1;
      let replaceTx = new Transaction().add(
        createReplaceIx(
          GummyrollCtx,
          payer,
          merkleRollKeypair.publicKey,
          proof.root,
          proof.leaf,
          newLeaf,
          proof.index,
          proof.proofNodes
        )
      );
      await GummyrollCtx.provider
        .send(replaceTx, [payer], {
          commitment: "confirmed",
        })
        .then(() => {
          console.log(
            `Replaced ${bs58.encode(proof.leaf)} with ${bs58.encode(
              newLeaf
            )}, index ${sample}`
          );
          updateTree(offChainMerkle, newLeaf, sample);
          if (!(sample in replacedInds)) replacedInds.push(sample);
        })
        .catch(async (x) => {
          console.log("Encountered error on ", bs58.encode(proof.leaf));
          failed += 1;
          await logStats(
            nftDb,
            offChainMerkle,
            appends,
            replaces,
            failed,
            replacedInds
          );
        });
    }
  }
}

async function logStats(
  nftDb: NFTDatabaseConnection,
  tree: Tree,
  appends: number,
  replaces: number,
  failed: number,
  replacedInds: Array<number>,
  repeat: boolean = true
) {
  let leafIdxs = await nftDb.getLeafIndices();
  let sequenceNumbers = Array.from(await nftDb.getSequenceNumbers()).sort(
    (a, b) => {
      return a - b;
    }
  );
  for (let i = 0; i < sequenceNumbers.length + 1; ++i) {
    let prev = sequenceNumbers[i];
    let curr = sequenceNumbers[i + 1];
    for (let j = 1; j < curr - prev; ++j) {
      console.log("Missing sequence number in DB: ", prev + j);
    }
  }
  let count = 0;
  console.log(
    replacedInds.sort((a, b) => {
      return a - b;
    })
  );
  for (const [nodeIdx, hash] of leafIdxs) {
    let proof = await nftDb.generateProof(nodeIdx, hash, false);
    let corruptedIdx = nodeIdx - (1 << Math.log2(nodeIdx));
    if (!nftDb.verifyProof(proof)) {
      console.log(
        `   ${corruptedIdx} proof verification failed, hash: ${bs58.encode(
          hash
        )}, stored hash: ${nftDb.tree.get(nodeIdx)}`
      );
      count++;
    }
    let offChainProof = getProofOfLeaf(tree, corruptedIdx);
    let root = offChainProof.pop().node;
    let proofNodes = offChainProof.map((x) => x.node);
    let newProof: Proof = {
      leaf: hash,
      root: root,
      index: nodeIdx,
      proofNodes: proofNodes,
    };
    if (!nftDb.verifyProof(newProof)) {
      console.log(
        `   OFF CHAIN ${corruptedIdx} proof verification failed, hash: ${bs58.encode(
          hash
        )}, stored hash: ${nftDb.tree.get(nodeIdx)}`
      );
    } 
  }
  console.log(`Verification failed for ${count}/${leafIdxs.length}`);
  console.log(`Stats:`);
  console.log(`   Appends: ${appends}`);
  console.log(`   Replace attempts: ${replaces}`);
  console.log(`   Failed replaces: ${failed}`);
  if (count > 0 && repeat) {
    console.log("Sleeping for the DB to recover");
    await new Promise((r) => setTimeout(r, 2000));
    console.log("Polling status again");
    await logStats(nftDb, tree, appends, replaces, failed, replacedInds, false);
  }
}

main()
  .then(() => {
    console.log("Done");
  })
  .catch((e) => {
    console.error(e);
  });
