import * as anchor from "@project-serum/anchor";
import { MerkleWallet } from "../target/types/merkle_wallet";
import { Program, BN, IdlAccounts } from "@project-serum/anchor";
import { Borsh } from "@metaplex-foundation/mpl-core";
import {
  MetadataDataData,
  DataV2,
  CreateMetadataV2,
  MetadataProgram,
  CreateMasterEditionV3,
} from "@metaplex-foundation/mpl-token-metadata";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  PayerTransactionHandler,
  defaultSendOptions,
} from "@metaplex-foundation/amman";
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
  const feepayer = Keypair.generate();

  it("Initialize start state", async () => {
    // Airdropping tokens to a payer.
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(payer.publicKey, 10000000000),
      "confirmed"
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(feepayer.publicKey, 10000000000),
      "confirmed"
    );
  });
  it("Create Merkle wallet", async () => {
    const [merkleWalletKey, bump] = await PublicKey.findProgramAddress(
      [Buffer.from("MERKLE"), payer.publicKey.toBuffer()],
      program.programId
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

    const URI = "uri";
    const NAME = "test";
    const SYMBOL = "sym";
    const SELLER_FEE_BASIS_POINTS = 10;
    const initMetadataData = new DataV2({
      uri: URI,
      name: NAME,
      symbol: SYMBOL,
      sellerFeeBasisPoints: SELLER_FEE_BASIS_POINTS,
      creators: null,
      collection: null,
      uses: null,
    });
    const transactionHandler = new PayerTransactionHandler(
      program.provider.connection,
      payer
    );

    let [metadataKey, _metadataBump] =
      await MetadataProgram.findMetadataAccount(mintKey);

    let [masterEditionKey, _masterEditionBump] =
      await MetadataProgram.findMasterEditionAccount(mintKey);

    let metadataTx = new CreateMetadataV2(
      { feePayer: payer.publicKey },
      {
        metadata: metadataKey,
        metadataData: initMetadataData,
        updateAuthority: payer.publicKey,
        mint: mintKey,
        mintAuthority: payer.publicKey,
      }
    );
    metadataTx.instructions[0].keys[3].isWritable = true;
    const metaplexTx = await program.provider.send(metadataTx, [payer], {
      commitment: "confirmed",
    });
    await logTx(program.provider, metaplexTx);
    let masterEditionTx = new CreateMasterEditionV3(
      { feePayer: payer.publicKey },
      {
        edition: masterEditionKey,
        metadata: metadataKey,
        updateAuthority: payer.publicKey,
        mint: mintKey,
        mintAuthority: payer.publicKey,
        maxSupply: new BN(1),
      }
    );
    masterEditionTx.instructions[0].keys[2].isWritable = true;
    masterEditionTx.instructions[0].keys[3].isWritable = true;
    masterEditionTx.instructions[0].keys[4].isWritable = true;
    masterEditionTx.instructions[0].keys[6].pubkey = TOKEN_PROGRAM_2022_ID;

    for (const k of masterEditionTx.instructions[0].keys) {
      console.log(k.pubkey.toBase58(), k.isSigner, k.isWritable);
    }
    console.log(masterEditionKey.toBase58())
    console.log(metadataKey.toBase58())
    console.log(payer.publicKey.toBase58())
    const metaplexMETx = await program.provider.send(masterEditionTx, [payer], {
      commitment: "confirmed",
    });
    await logTx(program.provider, metaplexMETx);
  });


  it("Compress NFT", async () => {
    // This part is going to suck...
  });

  it("Decompress NFT", async () => {
    // This part is going to suck...
  });
});
