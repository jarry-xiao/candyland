
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

const generateLeafNode = (seeds) => {
  let leaf = Buffer.alloc(32);
  for (const seed of seeds) {
    leaf = Buffer.from(keccak_256.digest([...leaf, ...seed]));
  }
  return leaf;
};

type Node = {
    inner: PublicKey
}

describe("gummyroll", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.Gummyroll as Program<Gummyroll>;
  const payer = Keypair.generate();
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
    // console.log("Hmmm tasty logs");
    // const seeds = [payer.publicKey.toBuffer(), Uint8Array.from([0])];
    // const seeds = [payer.publicKey.toBuffer()];
    // console.log("Seeds: ", seeds);

    // const [merkleWalletKey, bump] = await PublicKey.findProgramAddress(
    //     seeds,
    //     program.programId,
    // );
    const merkleRollKeypair = Keypair.generate();
    console.log("Payer key:", payer.publicKey);

    const requiredSpace = 43568 + 8;
    const allocAccountIx = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: merkleRollKeypair.publicKey,
        lamports: await program.provider.connection.getMinimumBalanceForRentExemption(requiredSpace),
        space: requiredSpace,
        programId: program.programId,
    });

    const initGummyrollIx = await program.instruction.initGummyroll(
        { inner: Array.from(payer.publicKey.toBytes()) },
        {
            accounts: {
                merkleRoll: merkleRollKeypair.publicKey,
                payer: payer.publicKey,
                systemProgram: anchor.web3.SystemProgram.programId,
            },
            signers: [payer],
        }
    );
    console.log("init gummy roll ix:", initGummyrollIx);

    const tx = new Transaction().add(allocAccountIx).add(initGummyrollIx);
    let txid = await program.provider.send(tx, [payer, merkleRollKeypair], {
        commitment: 'confirmed'
    })
    await logTx(program.provider, txid);
  });
});