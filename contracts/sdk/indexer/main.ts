import { Program, web3 } from "@project-serum/anchor";
import { bootstrap } from "./db";
import {
  createAppendIx,
  getMerkleRollAccountSize,
  Gummyroll,
} from "../gummyroll";
import * as crypto from 'crypto';
import * as anchor from "@project-serum/anchor";
import { Keypair, SystemProgram, Transaction } from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  getUpdatedBatch,
  updateMerkleRollLive,
  updateMerkleRollSnapshot,
} from "./indexerGummyroll";

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
  await GummyrollCtx.provider.send(tx, [payer, merkleRollKeypair], {
    commitment: "confirmed",
  });
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
  while (1) {
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
  }

  // TODO make sure that we can get proofs from the SQL table
}

main()
  .then(() => {
    console.log("Done");
  })
  .catch((e) => {
    console.error(e);
  });
