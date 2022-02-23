import * as anchor from "@project-serum/anchor";
import { MerkleWallet } from "../target/types/merkle_wallet";
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

describe("merkle-wallet", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.MerkleWallet as Program<MerkleWallet>;
  const payer = Keypair.generate();
  const feepayer = Keypair.generate();

  it("Initialize start state", async () => {
    // Airdropping tokens to a payer.
    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(payer.publicKey, 10000000000),
      "confirmed"
    );
    await program.provider.connection.confirmTransaction(
      await program.provider.connection.requestAirdrop(feepayer.publicKey, 10000000000),
      "confirmed"
    );
  });
  it("Create Merkle wallet", async () => {
    const [merkleWalletKey, bump] = await PublicKey.findProgramAddress(
      [Buffer.from("MERKLE"), payer.publicKey.toBuffer()],
      program.programId
    );
    const [merkleAuthKey, authBump] = await PublicKey.findProgramAddress(
      [Buffer.from("MERKLE")],
      program.programId
    );
    const tx = await program.rpc.initializeMerkleWallet({
      accounts: {
        merkleWallet: merkleWalletKey,
        payer: payer.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [payer],
    });
    await logTx(program.provider, tx);

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


    const mintTx = await program.transaction.mintNft({
      accounts: {
        merkleWallet: merkleWalletKey,
        mint: mintKey,
        tokenAccount: tokenAccountKey,
        payer: payer.publicKey,
        authority: merkleAuthKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_2022_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [payer],
    });
    // await logTx(provider, tx);

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


    let compressTx = await program.transaction.compressNft(
      new BN(0),
      payer.publicKey,
      new BN(0),
      [],
      {
        accounts: {
          merkleWallet: merkleWalletKey,
          tokenAccount: tokenAccountKey,
          mint: mintKey,
          authority: merkleAuthKey,
          metadata: metadataKey,
          masterEdition: masterEditionKey,
          owner: payer.publicKey,
          tokenProgram: TOKEN_PROGRAM_2022_ID,
          tokenMetadataProgram: MetadataProgram.PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [payer],
      }
    );

    let nftTx = new Transaction().add(mintTx).add(metadataTx).add(masterEditionTx).add(compressTx);

    let txid = await program.provider.send(nftTx, [payer], {
      commitment: "confirmed",
    });
    await logTx(program.provider, txid);

    // let metadataData = await program.provider.connection.getAccountInfo(
    //   metadataKey,
    //   "confirmed"
    // );
    // let masterEditionData = await program.provider.connection.getAccountInfo(
    //   masterEditionKey,
    //   "confirmed"
    // );

    // let masterEdition = MasterEditionV2Data.deserialize(masterEditionData.data);

    // let leaf = generateLeafNode([
    //   metadataData.data,
    //   masterEdition.maxSupply.toBuffer("le", 8),
    //   masterEdition.supply.toBuffer("le", 8),
    //   payer.publicKey.toBuffer(),
    //   new BN(0).toBuffer("le", 16),
    // ]);
    // console.log(leaf.join(" "));
  });

  it("Decompress NFT", async () => {
    // This part is going to suck...
  });
});
