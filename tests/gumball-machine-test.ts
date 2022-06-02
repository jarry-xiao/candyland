import * as anchor from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BN, Provider, Program } from "@project-serum/anchor";
import { Bubblegum } from "../target/types/bubblegum";
import { GumballMachine } from "../target/types/gumball_machine";
import { Gummyroll } from "../target/types/gummyroll";
import { Key, PROGRAM_ID } from "@metaplex-foundation/mpl-token-metadata";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  LAMPORTS_PER_SOL,
  TransactionInstruction,
} from "@solana/web3.js";
import { assert, expect } from "chai";

import { buildTree, Tree } from "./merkle-tree";
import {
  decodeMerkleRoll,
  getMerkleRollAccountSize,
  assertOnChainMerkleRollProperties
} from "./merkle-roll-serde";
import {
  decodeGumballMachine, OnChainGumballMachine
} from "./gumball-machine-serde";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { getAssociatedTokenAddress, createMint, getOrCreateAssociatedTokenAccount, mintTo, getAccount } from "../deps/solana-program-library/token/js/src";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  NATIVE_MINT
} from "@solana/spl-token";
import { logTx, num32ToBuffer } from "./utils";

// @ts-ignore
let GumballMachine;
let Bubblegum;
// @ts-ignore
let GummyrollProgramId;
let BubblegumProgramId;

describe("gumball-machine", () => {
  // Configure the client to use the local cluster.

  const payer = Keypair.generate();
  
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

  GumballMachine = anchor.workspace.GumballMachine as Program<GumballMachine>;
  Bubblegum = anchor.workspace.Bubblegum as Program<Bubblegum>;
  GummyrollProgramId = anchor.workspace.Gummyroll.programId;
  BubblegumProgramId = anchor.workspace.Bubblegum.programId;

  // TODO(sorend): consider removing and only using GumballMachineHeader type
  //               but this is cleaner for the test readability
  type InitGumballMachineProps = {
    urlBase: Buffer,
    nameBase: Buffer,
    symbol: Buffer,
    sellerFeeBasisPoints: number,
    isMutable: boolean,
    retainAuthority: boolean,
    price: BN,
    goLiveDate: BN,
    mint: PublicKey,
    botWallet: PublicKey,
    receiver: PublicKey,
    authority: PublicKey,
    collectionKey: PublicKey,
    creatorAddress: PublicKey,
    extensionLen: BN,
    maxMintSize: BN, 
    maxItems: BN
  }
  
  function assertGumballMachineHeaderProperties(gm: OnChainGumballMachine, expectedHeader: InitGumballMachineProps) {
    assert(
      gm.header.urlBase.equals(expectedHeader.urlBase),
      "Gumball Machine has incorrect url base"
    );
    assert(
      gm.header.nameBase.equals(expectedHeader.nameBase),
      "Gumball Machine has incorrect name base"
    );
    assert(
      gm.header.symbol.equals(expectedHeader.symbol),
      "Gumball Machine has incorrect symbol"
    );
    assert(
      gm.header.sellerFeeBasisPoints === expectedHeader.sellerFeeBasisPoints,
      "Gumball Machine has seller fee basis points"
    );
    assert(
      gm.header.isMutable === expectedHeader.isMutable,
      "Gumball Machine has incorrect isMutable"
    );
    assert(
      gm.header.retainAuthority === expectedHeader.retainAuthority,
      "Gumball Machine has incorrect retainAuthority"
    );
    assert(
      gm.header.price.eq(expectedHeader.price),
      "Gumball Machine has incorrect price"
    );
    assert(
      gm.header.goLiveDate.eq(expectedHeader.goLiveDate),
      "Gumball Machine has incorrect goLiveDate"
    );
    assert(
      gm.header.mint.equals(expectedHeader.mint),
      "Gumball Machine set with incorrect mint"
    );
    assert(
      gm.header.botWallet.equals(expectedHeader.botWallet),
      "Gumball Machine set with incorrect botWallet"
    );
    assert(
      gm.header.receiver.equals(expectedHeader.receiver),
      "Gumball Machine set with incorrect receiver"
    );
    assert(
      gm.header.authority.equals(expectedHeader.authority),
      "Gumball Machine set with incorrect authority"
    );
    assert(
      gm.header.collectionKey.equals(expectedHeader.collectionKey),
      "Gumball Machine set with incorrect collectionKey"
    );
    assert(
      gm.header.creatorAddress.equals(expectedHeader.creatorAddress),
      "Gumball Machine set with incorrect creatorAddress"
    );
    assert(
      gm.header.extensionLen.eq(expectedHeader.extensionLen),
      "Gumball Machine has incorrect extensionLen"
    );
    assert(
      gm.header.maxMintSize.eq(expectedHeader.maxMintSize),
      "Gumball Machine has incorrect maxMintSize"
    );
    assert(
      gm.header.maxItems.eq(expectedHeader.maxItems),
      "Gumball Machine has incorrect max items"
    );
  }

  function assertGumballMachineConfigProperties(gm: OnChainGumballMachine, expectedIndexArray: Buffer, expectedConfigLines: Buffer, onChainConfigLinesNumBytes: number) {
    assert(
      gm.configData.indexArray.equals(expectedIndexArray),
      "Onchain index array doesn't match expectation"
    )

    // Calculate full-sized on-chain config bytes buffer, we must null pad the buffer up to the end of the account size
    const numExpectedInitializedBytesInConfig = expectedConfigLines.byteLength
    const bufferOfNonInitializedConfigLineBytes = Buffer.from("\0".repeat(onChainConfigLinesNumBytes-numExpectedInitializedBytesInConfig))
    const actualExpectedConfigLinesBuffer = Buffer.concat([expectedConfigLines, bufferOfNonInitializedConfigLineBytes])
    assert(
      gm.configData.configLines.equals(actualExpectedConfigLinesBuffer),
      "Config lines on gumball machine do not match expectation"
    )
  }

  async function getBubblegumAuthorityPDAKey(merkleRollPubKey: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
      [merkleRollPubKey.toBuffer()],
      BubblegumProgramId
    );
    return bubblegumAuthorityPDAKey
  }

  async function getWillyWinkaPDAKey(gumballMachinePubkey: PublicKey) {
    const [willyWonkaPDAKey] = await PublicKey.findProgramAddress(
      [gumballMachinePubkey.toBuffer()],
      GumballMachine.programId
    );
    return willyWonkaPDAKey
  }

  async function initializeGumballMachine(
    payer: Keypair,
    gumballMachineAcctKeypair: Keypair,
    gumballMachineAcctSize: number,
    merkleRollKeypair: Keypair,
    merkleRollAccountSize: number,
    desiredGumballMachineHeader: InitGumballMachineProps,
    maxDepth: number,
    maxBufferSize: number
  ) {

    const allocGumballMachineAcctInstr = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: gumballMachineAcctKeypair.publicKey,
      lamports:
        await GumballMachine.provider.connection.getMinimumBalanceForRentExemption(
          gumballMachineAcctSize
        ),
      space: gumballMachineAcctSize,
      programId: GumballMachine.programId,
    });

    const willyWonkaPDAKey = await getWillyWinkaPDAKey(gumballMachineAcctKeypair.publicKey);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey);

    const allocMerkleRollAcctInstr = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: merkleRollKeypair.publicKey,
      lamports:
        await GumballMachine.provider.connection.getMinimumBalanceForRentExemption(
          merkleRollAccountSize
        ),
      space: merkleRollAccountSize,
      programId: GummyrollProgramId,
    });

    const initializeGumballMachineInstr = GumballMachine.instruction.initializeGumballMachine(
      maxDepth,
      maxBufferSize,
      desiredGumballMachineHeader.urlBase, 
      desiredGumballMachineHeader.nameBase,
      desiredGumballMachineHeader.symbol,
      desiredGumballMachineHeader.sellerFeeBasisPoints,
      desiredGumballMachineHeader.isMutable,
      desiredGumballMachineHeader.retainAuthority,
      desiredGumballMachineHeader.price,
      desiredGumballMachineHeader.goLiveDate,
      desiredGumballMachineHeader.botWallet,
      desiredGumballMachineHeader.receiver,
      desiredGumballMachineHeader.authority, 
      desiredGumballMachineHeader.collectionKey,
      desiredGumballMachineHeader.extensionLen,
      desiredGumballMachineHeader.maxMintSize,
      desiredGumballMachineHeader.maxItems,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          creator: payer.publicKey,
          mint: desiredGumballMachineHeader.mint,
          willyWonka: willyWonkaPDAKey,
          bubblegumAuthority: bubblegumAuthorityPDAKey,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId
        },
        signers: [payer],
      }
    );

    const tx = new Transaction().add(allocGumballMachineAcctInstr).add(allocMerkleRollAcctInstr).add(initializeGumballMachineInstr)
    await GumballMachine.provider.send(tx, [payer, gumballMachineAcctKeypair, merkleRollKeypair], {
      commitment: "confirmed",
    });

    const tree = buildTree(Array(2 ** maxDepth).fill(Buffer.alloc(32)));
    await assertOnChainMerkleRollProperties(GumballMachine.provider.connection, maxDepth, maxBufferSize, bubblegumAuthorityPDAKey, new PublicKey(tree.root), merkleRollKeypair.publicKey);

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKeypair.publicKey
    );

    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    assertGumballMachineHeaderProperties(gumballMachine, desiredGumballMachineHeader);
  }

  async function addConfigLines(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize: number,
    gumballMachineAcctConfigIndexArrSize: number,
    gumballMachineAcctConfigLinesSize: number,
    configLinesToAdd: Buffer,
    allExpectedInitializedConfigLines: Buffer
  ) {
    const addConfigLinesInstr = GumballMachine.instruction.addConfigLines(
      configLinesToAdd,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKey,
          authority: authority.publicKey
        },
        signers: [authority]
      }
    )
    const tx = new Transaction().add(addConfigLinesInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);

    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [...Array(gumballMachineAcctConfigIndexArrSize/4).keys()].reduce(
      (prevVal, curVal) => Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    )

    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, allExpectedInitializedConfigLines, gumballMachineAcctConfigLinesSize);
  }

  async function updateConfigLines(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    gumballMachineAcctConfigIndexArrSize: number,
    gumballMachineAcctConfigLinesSize: number,
    updatedConfigLines: Buffer,
    allExpectedInitializedConfigLines: Buffer,
    indexOfFirstLineToUpdate: BN
  ) {
    const updateConfigLinesInstr = GumballMachine.instruction.updateConfigLines(
      indexOfFirstLineToUpdate,
      updatedConfigLines,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKey,
          authority: authority.publicKey
        },
        signers: [authority]
      }
    );

    const tx = new Transaction().add(updateConfigLinesInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    
    // Create the expected buffer for the indices of the account
    const expectedIndexArrBuffer = [...Array(gumballMachineAcctConfigIndexArrSize/4).keys()].reduce(
      (prevVal, curVal) => Buffer.concat([prevVal, Buffer.from(num32ToBuffer(curVal))]),
      Buffer.from([])
    )
    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, allExpectedInitializedConfigLines, gumballMachineAcctConfigLinesSize);
  }

  async function updateHeaderMetadata(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    newHeader: InitGumballMachineProps,
  ) {
    const updateHeaderMetadataInstr = GumballMachine.instruction.updateHeaderMetadata(
      newHeader.urlBase,
      newHeader.nameBase,
      newHeader.symbol,
      newHeader.sellerFeeBasisPoints,
      newHeader.isMutable,
      newHeader.retainAuthority,
      newHeader.price,
      newHeader.goLiveDate,
      newHeader.botWallet,
      newHeader.authority, 
      newHeader.maxMintSize,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKey,
          authority: authority.publicKey
        },
        signers: [authority]
      }
    );

    const tx = new Transaction().add(updateHeaderMetadataInstr)
    await GumballMachine.provider.send(tx, [authority], {
      commitment: "confirmed",
    });

    const onChainGumballMachineAccount = await GumballMachine.provider.connection.getAccountInfo(
      gumballMachineAcctKey
    );
    const gumballMachine = decodeGumballMachine(onChainGumballMachineAccount.data, gumballMachineAcctSize);
    assertGumballMachineHeaderProperties(gumballMachine, newHeader);
  }

  async function getDispenseNFTForSolInstruction(
    numNFTs: BN,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ): Promise<TransactionInstruction> {
    const willyWonkaPDAKey = await getWillyWinkaPDAKey(gumballMachineAcctKeypair.publicKey);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey);
    const dispenseInstr = GumballMachine.instruction.dispenseNftSol(
      numNFTs,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          payer: payer.publicKey,
          receiver: receiver,
          systemProgram: SystemProgram.programId,
          willyWonka: willyWonkaPDAKey,
          recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
          instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
          bubblegumAuthority: bubblegumAuthorityPDAKey,
          nonce: noncePDAKey,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId
        },
        signers: [payer]
      }
    );
    return dispenseInstr;
  }

  async function dispenseCompressedNFTForSol(
    numNFTs: BN,
    payer: Keypair,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ) {
    const dispenseInstr = await getDispenseNFTForSolInstruction(
      numNFTs,
      payer,
      receiver,
      gumballMachineAcctKeypair,
      merkleRollKeypair,
      noncePDAKey
    );
    const tx = new Transaction().add(dispenseInstr);
    await GumballMachine.provider.send(tx, [payer], {
      commitment: "confirmed",
    });

    // TODO(sorend): assert that the effects of the mint are as expected             
  }

  async function dispenseCompressedNFTForTokens(
    numNFTs: BN,
    payer: Keypair,
    payerTokens: PublicKey,
    receiver: PublicKey,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ) {
    const willyWonkaPDAKey = await getWillyWinkaPDAKey(gumballMachineAcctKeypair.publicKey);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey);
    const dispenseInstr = GumballMachine.instruction.dispenseNftToken(
      numNFTs,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          payer: payer.publicKey,
          payerTokens,
          receiver,
          tokenProgram: TOKEN_PROGRAM_ID,
          willyWonka: willyWonkaPDAKey,
          recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
          instructionSysvarAccount: SYSVAR_INSTRUCTIONS_PUBKEY,
          bubblegumAuthority: bubblegumAuthorityPDAKey,
          nonce: noncePDAKey,
          gummyroll: GummyrollProgramId,
          merkleSlab: merkleRollKeypair.publicKey,
          bubblegum: BubblegumProgramId
        },
        signers: [payer]
      }
    );

    const tx = new Transaction().add(dispenseInstr);
    await GumballMachine.provider.send(tx, [payer], {
      commitment: "confirmed",
    });

    // TODO(sorend): assert that the effects of the mint are as expected             
  }

  async function destroyGumballMachine(
    gumballMachineAcctKeypair: Keypair,
    authorityKeypair: Keypair
  ) {
    const originalGumballMachineAcctBalance = await connection.getBalance(gumballMachineAcctKeypair.publicKey);
    const originalAuthorityAcctBalance = await connection.getBalance(authorityKeypair.publicKey);
    const destroyInstr = GumballMachine.instruction.destroy(
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          authority: authorityKeypair.publicKey
        },
        signers: [authorityKeypair]
      }
    );

    const tx = new Transaction().add(destroyInstr);
    await GumballMachine.provider.send(tx, [authorityKeypair], {
      commitment: "confirmed",
    });

    assert(
      0 === await connection.getBalance(gumballMachineAcctKeypair.publicKey),
      "Failed to remove lamports from gumball machine acct"
    );

    const expectedAuthorityAcctBalance = originalAuthorityAcctBalance + originalGumballMachineAcctBalance
    assert(
      expectedAuthorityAcctBalance === await connection.getBalance(authorityKeypair.publicKey),
      "Failed to transfer correct balance to authority"
    );
  }

  describe("Testing gumball-machine", async () => {
    let baseGumballMachineHeader: InitGumballMachineProps;
    let creatorAddress: Keypair;
    let gumballMachineAcctKeypair: Keypair;
    let merkleRollKeypair: Keypair;
    let noncePDAKey: PublicKey;
    let nftBuyer: Keypair;
    const GUMBALL_MACHINE_ACCT_HEADER_SIZE = 384;
    const GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE = 1000;
    const GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = 7000;
    const GUMBALL_MACHINE_ACCT_SIZE = GUMBALL_MACHINE_ACCT_HEADER_SIZE + GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE + GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE;
    const MERKLE_ROLL_ACCT_SIZE = getMerkleRollAccountSize(3,8);

    before(async () => {

      // Give funds to the payer for the whole suite
      await GumballMachine.provider.connection.confirmTransaction(
        await GumballMachine.provider.connection.requestAirdrop(payer.publicKey, 25e9),
        "confirmed"
      );

      [noncePDAKey] = await PublicKey.findProgramAddress(
        [Buffer.from("bubblegum")],
        BubblegumProgramId
      );

      // Attempt to initialize the nonce account. Since localnet is not torn down between suites,
      // there is some shared state. Specifically, the Bubblegum suite may initialize this account
      // if it is run first. Thus even in the case of an error, we proceed.
      try {
        await Bubblegum.rpc.initializeNonce({
          accounts: {
            nonce: noncePDAKey,
            payer: payer.publicKey,
            systemProgram: SystemProgram.programId,
          },
          signers: [payer],
        });
      } catch(e) {
        console.log("Bubblegum nonce PDA already initialized by other suite")
      }
    });

    describe("native sol projects", async () => {
      let creatorPaymentWallet: Keypair;
      beforeEach(async () => {
        creatorAddress = Keypair.generate();
        creatorPaymentWallet = Keypair.generate();
        nftBuyer = Keypair.generate();
        gumballMachineAcctKeypair = Keypair.generate();
        merkleRollKeypair = Keypair.generate();

        baseGumballMachineHeader = {
          urlBase: Buffer.from("https://arweave.net/Rmg4pcIv-0FQ7M7X838p2r592Q4NU63Fj7o7XsvBHEEl"),
          nameBase: Buffer.from("zfgfsxrwieciemyavrpkuqehkmhqmnim"),
          symbol: Buffer.from("pehjjqmrjpfcnttlierdqkxjueqjqjsf"), 
          sellerFeeBasisPoints: 100,
          isMutable: true,
          retainAuthority: true,
          price: new BN(10),
          goLiveDate: new BN(1234.0),
          mint: NATIVE_MINT,
          botWallet: Keypair.generate().publicKey,
          receiver: creatorPaymentWallet.publicKey,
          authority: creatorAddress.publicKey,
          collectionKey: SystemProgram.programId, // 0x0 -> no collection key
          creatorAddress: creatorAddress.publicKey,
          extensionLen: new BN(28),
          maxMintSize: new BN(10),
          maxItems: new BN(250)
        };
  
        // Give creator enough funds to produce accounts for NFT
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(creatorAddress.publicKey, LAMPORTS_PER_SOL),
          "confirmed"
        );
  
        await initializeGumballMachine(creatorAddress, gumballMachineAcctKeypair, GUMBALL_MACHINE_ACCT_SIZE, merkleRollKeypair, MERKLE_ROLL_ACCT_SIZE, baseGumballMachineHeader, 3, 8);
        await addConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc"), Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc"));
      });
      describe("dispense nft sol instruction", async () => {
        beforeEach(async () => {
          // Give the recipient address enough money to not get rent exempt
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(baseGumballMachineHeader.receiver, LAMPORTS_PER_SOL),
            "confirmed"
          );
  
          // Fund the NFT Buyer
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(nftBuyer.publicKey, LAMPORTS_PER_SOL),
            "confirmed"
          );
        });
        describe("transaction atomicity attacks fail", async () => {
          it("Cannot dispense NFT for SOL with subsequent instructions in transaction", async () => {
            const dispenseNFTForSolInstr = await getDispenseNFTForSolInstruction(new BN(1), nftBuyer, baseGumballMachineHeader.receiver, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
            const dummyNewAcctKeypair = Keypair.generate();
            const dummyInstr = SystemProgram.createAccount({
              fromPubkey: payer.publicKey,
              newAccountPubkey: dummyNewAcctKeypair.publicKey,
              lamports: 10000000,
              space: 100,
              programId: GumballMachine.programId,
            });
            const tx = new Transaction().add(dispenseNFTForSolInstr).add(dummyInstr);
            try {
              await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
                commitment: "confirmed",
              })
              assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
            } catch(e) {}
          });
          it("Cannot dispense NFT for SOL with prior instructions in transaction", async () => {
            const dispenseNFTForSolInstr = await getDispenseNFTForSolInstruction(new BN(1), nftBuyer, baseGumballMachineHeader.receiver, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
            const dummyNewAcctKeypair = Keypair.generate();
            const dummyInstr = SystemProgram.createAccount({
              fromPubkey: payer.publicKey,
              newAccountPubkey: dummyNewAcctKeypair.publicKey,
              lamports: 10000000,
              space: 100,
              programId: GumballMachine.programId,
            });
            const tx = new Transaction().add(dummyInstr).add(dispenseNFTForSolInstr);
            try {
              await GumballMachine.provider.send(tx, [nftBuyer, payer, dummyNewAcctKeypair], {
                commitment: "confirmed",
              })
              assert(false, "Dispense should fail when part of transaction with multiple instructions, but it succeeded");
            } catch(e) {}
          });
        });
        it("Can dispense single NFT paid in sol", async () => {
          // Give the recipient address enough money to not get rent exempt
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(baseGumballMachineHeader.receiver, LAMPORTS_PER_SOL),
            "confirmed"
          );
  
          // Fund the NFT Buyer
          await GumballMachine.provider.connection.confirmTransaction(
            await GumballMachine.provider.connection.requestAirdrop(nftBuyer.publicKey, LAMPORTS_PER_SOL),
            "confirmed"
          );
    
          const nftBuyerBalanceBeforePurchase = await connection.getBalance(nftBuyer.publicKey);
          const creatorBalanceBeforePurchase = await connection.getBalance(baseGumballMachineHeader.receiver);
  
          // Purchase the compressed NFT with SOL
          await dispenseCompressedNFTForSol(new BN(1), nftBuyer, baseGumballMachineHeader.receiver, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
          const nftBuyerBalanceAfterPurchase = await connection.getBalance(nftBuyer.publicKey);
          const creatorBalanceAfterPurchase = await connection.getBalance(baseGumballMachineHeader.receiver);
    
          // Assert on how the creator and buyer's balances changed
          assert(
            await creatorBalanceAfterPurchase === (creatorBalanceBeforePurchase + baseGumballMachineHeader.price.toNumber()),
            "Creator balance did not update as expected after NFT purchase"
          );
    
          assert(
            await nftBuyerBalanceAfterPurchase === (nftBuyerBalanceBeforePurchase - baseGumballMachineHeader.price.toNumber()),
            "NFT purchaser balance did not decrease as expected after NFT purchase"
          );
        });
      });
      // @notice: We only test admin instructions on SOL projects because they are completely (for now) independent of project mint
      describe("admin instructions", async () => {
        it("Can update config lines", async () => {
          await updateConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"), Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"), new BN(0));
        });
        it("Can update gumball header", async () => {
          const newGumballMachineHeader: InitGumballMachineProps = {
            urlBase: Buffer.from("https://arweave.net/bzdjillretjcraaxawlnhqrhmexzbsixyajrlzhfcvcc"),
            nameBase: Buffer.from("wmqeslreeondhmcmtfebrwqnqcoasbye"),
            symbol: Buffer.from("wmqeslreeondhmcmtfebrwqnqcoasbye"), 
            sellerFeeBasisPoints: 50,
            isMutable: false,
            retainAuthority: false,
            price: new BN(100),
            goLiveDate: new BN(5678.0),
            mint: baseGumballMachineHeader.mint,                     // Cannot be modified after init
            botWallet: Keypair.generate().publicKey,
            receiver: baseGumballMachineHeader.receiver,             // FOR NOW: Cannot be modified after init
            authority: Keypair.generate().publicKey,
            collectionKey: baseGumballMachineHeader.collectionKey,   // Cannot be modified after init
            creatorAddress: baseGumballMachineHeader.creatorAddress, // Cannot be modified after init
            extensionLen: baseGumballMachineHeader.extensionLen,     // Cannot be modified after init
            maxMintSize: new BN(15),
            maxItems: baseGumballMachineHeader.maxItems              // Cannot be modified after init
          };
          await updateHeaderMetadata(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, newGumballMachineHeader);
        });
        it("Can destroy gumball machine and reclaim lamports", async () => {
          await destroyGumballMachine(gumballMachineAcctKeypair, creatorAddress);
        });
      });
    });

    describe("spl token projects", async () => {
      let nftBuyer: Keypair;
      beforeEach(async () => {
        creatorAddress = Keypair.generate();
        // Give creator enough funds to produce accounts for gumball-machine
        await GumballMachine.provider.connection.confirmTransaction(
          await GumballMachine.provider.connection.requestAirdrop(creatorAddress.publicKey, 4 * LAMPORTS_PER_SOL),
          "confirmed"
        );
        gumballMachineAcctKeypair = Keypair.generate();
        merkleRollKeypair = Keypair.generate();
        nftBuyer = Keypair.generate();
      });
      it("Can dispense multiple NFTs paid in token", async () => {
        const someMint = await createMint(connection, payer, payer.publicKey, null, 9);
        let creatorReceiverTokenAccount = await getOrCreateAssociatedTokenAccount(connection, payer, someMint, creatorAddress.publicKey);
  
        const botWallet = Keypair.generate();
        baseGumballMachineHeader = {
          urlBase: Buffer.from("https://arweave.net/Rmg4pcIv-0FQ7M7X838p2r592Q4NU63Fj7o7XsvBHEEl"),
          nameBase: Buffer.from("zfgfsxrwieciemyavrpkuqehkmhqmnim"),
          symbol: Buffer.from("pehjjqmrjpfcnttlierdqkxjueqjqjsf"), 
          sellerFeeBasisPoints: 100,
          isMutable: true,
          retainAuthority: true,
          price: new BN(10),
          goLiveDate: new BN(1234.0),
          mint: someMint,
          botWallet: botWallet.publicKey,
          receiver: creatorReceiverTokenAccount.address,
          authority: creatorAddress.publicKey,
          collectionKey: SystemProgram.programId, // 0x0 -> no collection
          creatorAddress: creatorAddress.publicKey,
          extensionLen: new BN(28),
          maxMintSize: new BN(10),
          maxItems: new BN(250)
        };
  
        await initializeGumballMachine(creatorAddress, gumballMachineAcctKeypair, GUMBALL_MACHINE_ACCT_SIZE, merkleRollKeypair, MERKLE_ROLL_ACCT_SIZE, baseGumballMachineHeader, 3, 8);
        await addConfigLines(creatorAddress, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"), Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc" + "aauvnpwncgchwnbqfpbtdlcpdthc"));
  
        // Create and fund the NFT pruchaser
        const nftBuyerTokenAccount = await getOrCreateAssociatedTokenAccount(connection, payer, someMint, nftBuyer.publicKey);
        await mintTo(connection, payer, someMint, nftBuyerTokenAccount.address, payer, 50);
        let buyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        await dispenseCompressedNFTForTokens(new BN(1), nftBuyer, nftBuyerTokenAccount.address, creatorReceiverTokenAccount.address, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
  
        let newCreatorTokenAccount = await getAccount(connection, creatorReceiverTokenAccount.address);
        let newBuyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        
        assert(
          Number(newCreatorTokenAccount.amount) === Number(creatorReceiverTokenAccount.amount) + baseGumballMachineHeader.price.toNumber(),
          "The creator did not receive their payment as expected"
        );
  
        assert(
          Number(newBuyerTokenAccount.amount) === Number(buyerTokenAccount.amount) - baseGumballMachineHeader.price.toNumber(),
          "The nft buyer did not pay for the nft as expected"
        );

        await dispenseCompressedNFTForTokens(new BN(1), nftBuyer, nftBuyerTokenAccount.address, creatorReceiverTokenAccount.address, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);

        creatorReceiverTokenAccount = newCreatorTokenAccount;
        buyerTokenAccount = newBuyerTokenAccount;
        newCreatorTokenAccount = await getAccount(connection, creatorReceiverTokenAccount.address);
        newBuyerTokenAccount = await getAccount(connection, nftBuyerTokenAccount.address);
        
        assert(
          Number(newCreatorTokenAccount.amount) === Number(creatorReceiverTokenAccount.amount) + baseGumballMachineHeader.price.toNumber(),
          "The creator did not receive their payment as expected"
        );
  
        assert(
          Number(newBuyerTokenAccount.amount) === Number(buyerTokenAccount.amount) - baseGumballMachineHeader.price.toNumber(),
          "The nft buyer did not pay for the nft as expected"
        );
      });
    });
  });
});
