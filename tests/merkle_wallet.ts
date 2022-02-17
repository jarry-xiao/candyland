import * as anchor from "@project-serum/anchor";
import { MerkleWallet } from "../target/types/merkle_wallet";
import { Program, BN, IdlAccounts } from "@project-serum/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  Token,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";

const TOKEN_PROGRAM_2022_ID = new PublicKey("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

const logTx = async (provider, tx) => {
  await provider.connection.confirmTransaction(tx, "confirmed");
  console.log(
    (await provider.connection.getConfirmedTransaction(tx, "confirmed")).meta
      .logMessages
  );
};

describe("merkle-wallet", () => {
  // Configure the client to use the local cluster.
  const defaultProvider = anchor.Provider.env();
  const provider = new anchor.Provider(
    defaultProvider.connection,
    defaultProvider.wallet,
    { commitment: "confirmed" }
  );
  const idl = JSON.parse(
    require("fs").readFileSync("./target/idl/merkle_wallet.json", "utf8")
  );

  // Address of the deployed program.
  const programId = new anchor.web3.PublicKey(
    "7iNZXYZDn1127tRp1GSe3W3zGqGNdw16SiCwANNfTqXH"
  );

  // Generate the program client from IDL.
  const program = new anchor.Program(idl, programId, provider);
  const payer = Keypair.generate();

  it("Initialize start state", async () => {
    // Airdropping tokens to a payer.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 10000000000),
      "confirmed"
    );
  });
  it("Create Merkle wallet", async () => {
    const [merkleWalletKey, bump] = (
      await PublicKey.findProgramAddress(
        [Buffer.from("MERKLE"), payer.publicKey.toBuffer()],
        program.programId
      )
    );
    let tx = await program.rpc.initializeMerkleWallet({
      accounts: {
        merkleWallet: merkleWalletKey,
        payer: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [payer],
    });
    await logTx(provider, tx);

    let merkleWallet = await program.account.merkleWallet.fetch(
      merkleWalletKey
    );
    assert.ok(merkleWallet.counter.toNumber() == 0);
    assert.ok(merkleWallet.bump == bump);

    const mintKey = (
      await PublicKey.findProgramAddress(
        [payer.publicKey.toBuffer(), merkleWallet.counter.toBuffer("le", 16)],
        program.programId
      )
    )[0];

    const tokenAccountKey = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_2022_ID,
      mintKey,
      payer.publicKey
    );

    console.log("token program", TOKEN_PROGRAM_2022_ID.toBase58());
    console.log("merkleWallet", merkleWalletKey.toBase58());
    console.log("mint", mintKey.toBase58());
    console.log("token account", tokenAccountKey.toBase58());

    tx = await program.rpc.mintNft({
      accounts: {
        merkleWallet: merkleWalletKey,
        mint: mintKey,
        tokenAccount: tokenAccountKey,
        payer: payer.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_2022_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [payer],
    });
    await logTx(provider, tx);
  });

  it("Initialize candy machine", async () => {
    // This part is going to suck... 
  });

  it("Compress NFT", async () => {
    // This part is going to suck... 
  });

  it("Decompress NFT", async () => {
    // This part is going to suck... 
  });
});
