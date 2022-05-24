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
} from "@solana/web3.js";
import { assert } from "chai";

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
import { getAssociatedTokenAddress } from "../deps/solana-program-library/token/js/src";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
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
  const wrappedSolPubKey = new PublicKey("So11111111111111111111111111111111111111112"); // TODO(sorend): associate with real mint account when payments implemented
  
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
      desiredGumballMachineHeader.authority, 
      desiredGumballMachineHeader.collectionKey,
      desiredGumballMachineHeader.extensionLen,
      desiredGumballMachineHeader.maxMintSize,
      desiredGumballMachineHeader.maxItems,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          creator: payer.publicKey,
          mint: wrappedSolPubKey,
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
    configLines: Buffer
  ) {
    const addConfigLinesInstr = GumballMachine.instruction.addConfigLines(
      configLines,
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

    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, configLines, gumballMachineAcctConfigLinesSize);
  }

  async function updateConfigLines(
    authority: Keypair,
    gumballMachineAcctKey: PublicKey,
    gumballMachineAcctSize,
    gumballMachineAcctConfigIndexArrSize: number,
    gumballMachineAcctConfigLinesSize: number,
    updatedConfigLines: Buffer,
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
    assertGumballMachineConfigProperties(gumballMachine, expectedIndexArrBuffer, updatedConfigLines, gumballMachineAcctConfigLinesSize);
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

  async function dispenseCompressedNFT(
    numNFTs: BN,
    payer: Keypair,
    gumballMachineAcctKeypair: Keypair,
    merkleRollKeypair: Keypair,
    noncePDAKey: PublicKey
  ) {
    const willyWonkaPDAKey = await getWillyWinkaPDAKey(gumballMachineAcctKeypair.publicKey);
    const bubblegumAuthorityPDAKey = await getBubblegumAuthorityPDAKey(merkleRollKeypair.publicKey);
    const dispenseInstr = GumballMachine.instruction.dispense(
      numNFTs,
      {
        accounts: {
          gumballMachine: gumballMachineAcctKeypair.publicKey,
          payer: payer.publicKey,
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

    const approxExpectedAuthorityAcctBalance = originalAuthorityAcctBalance + originalGumballMachineAcctBalance
    assert(
      Math.abs(approxExpectedAuthorityAcctBalance - await connection.getBalance(authorityKeypair.publicKey)) < 10000,
      "Failed to transfer correct balance to authority"
    );
  }

  describe("Testing gumball_machine", async () => {
    const baseGumballMachineHeader: InitGumballMachineProps = {
      urlBase: Buffer.from("https://arweave.net/Rmg4pcIv-0FQ7M7X838p2r592Q4NU63Fj7o7XsvBHEEl"),
      nameBase: Buffer.from("zfgfsxrwieciemyavrpkuqehkmhqmnim"),
      symbol: Buffer.from("pehjjqmrjpfcnttlierdqkxjueqjqjsf"), 
      sellerFeeBasisPoints: 100,
      isMutable: true,
      retainAuthority: true,
      price: new BN(10),
      goLiveDate: new BN(1234.0),
      mint: wrappedSolPubKey,
      botWallet: Keypair.generate().publicKey,
      authority: payer.publicKey,
      collectionKey: payer.publicKey,
      creatorAddress: payer.publicKey,
      extensionLen: new BN(28),
      maxMintSize: new BN(10),
      maxItems: new BN(250)
    };

    let gumballMachineAcctKeypair;
    let merkleRollKeypair;
    let noncePDAKey;
    const GUMBALL_MACHINE_ACCT_SIZE = 8352;
    const GUMBALL_MACHINE_ACCT_HEADER_SIZE = 352;
    const GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE = 1000;
    const GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE = 7000;
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

    beforeEach(async () => {
      gumballMachineAcctKeypair = Keypair.generate();
      merkleRollKeypair = Keypair.generate();
      await initializeGumballMachine(payer, gumballMachineAcctKeypair, GUMBALL_MACHINE_ACCT_SIZE, merkleRollKeypair, MERKLE_ROLL_ACCT_SIZE, baseGumballMachineHeader, 3, 8);
      await addConfigLines(payer, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("uluvnpwncgchwnbqfpbtdlcpdthc"));
    });

    it("Can update config lines", async () => {
      await updateConfigLines(payer, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_INDEX_ARRAY_SIZE, GUMBALL_MACHINE_ACCT_CONFIG_LINES_SIZE, Buffer.from("aaavnpwncgchwnbqfpbtdlcpdaaa"), new BN(0));
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
        mint: wrappedSolPubKey,          // Cannot be modified after init
        botWallet: Keypair.generate().publicKey,
        authority: Keypair.generate().publicKey,
        collectionKey: payer.publicKey,  // Cannot be modified after init
        creatorAddress: payer.publicKey, // Cannot be modified after init
        extensionLen: new BN(28),        // Cannot be modified after init
        maxMintSize: new BN(15),
        maxItems: new BN(250)            // Cannot be modified after init
      };
      await updateHeaderMetadata(payer, gumballMachineAcctKeypair.publicKey, GUMBALL_MACHINE_ACCT_SIZE, newGumballMachineHeader);
    })
    it("Can destroy gumball machine and reclaim lamports", async () => {
      await destroyGumballMachine(gumballMachineAcctKeypair, payer);
    })
    it("Can dispense single NFT", async () => {
      await dispenseCompressedNFT(new BN(1), payer, gumballMachineAcctKeypair, merkleRollKeypair, noncePDAKey);
    });
  });
});
