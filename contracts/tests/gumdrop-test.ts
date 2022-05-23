import { PublicKey, Connection, Keypair, Transaction, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import * as anchor from "@project-serum/anchor";
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Gumdrop } from "../target/types/gumdrop";
import { MerkleTree } from './gumdropTree';
import { BinaryWriter } from 'borsh'

async function delay(ms: number) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function succeedOrThrow(txId: string, connection: Connection) {
  const err = (await connection.confirmTransaction(txId, "confirmed")).value.err
  if (err) {
    throw new Error(`${txId} failed: \n${JSON.stringify(err)}\n`);
  }
}

const GUMMYROLL_PROGRAM_ID = new PublicKey("GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD");
const BUBBLEGUM_PROGRAM_ID = new PublicKey("BGUMzZr2wWfD2yzrXFEWTK2HbdYhqQCP2EZoPEkZBD6o");

function getMerkleRollAccountSize(maxDepth: number, maxBufferSize: number): number {
  let headerSize = 8 + 32 + 32;
  let changeLogSize = (maxDepth * 32 + 32 + 4 + 4) * maxBufferSize;
  let rightMostPathSize = maxDepth * 32 + 32 + 4 + 4;
  let merkleRollSize = 8 + 8 + 16 + changeLogSize + rightMostPathSize;
  return merkleRollSize + headerSize;
}

async function initMerkleTreeInstruction(
  maxDepth: number,
  maxBufferSize: number,
  connection: Connection,
  merkleRoll: PublicKey,
  payer: PublicKey,
): Promise<TransactionInstruction> {
  const requiredSpace = getMerkleRollAccountSize(maxDepth, maxBufferSize);
  return SystemProgram.createAccount({
    fromPubkey: payer,
    newAccountPubkey: merkleRoll,
    lamports:
      await connection.getMinimumBalanceForRentExemption(
        requiredSpace
      ),
    space: requiredSpace,
    programId: GUMMYROLL_PROGRAM_ID,
  });
}

async function getDistributor(payer: PublicKey, gumdropId: PublicKey): Promise<[PublicKey, number]> {
  return await PublicKey.findProgramAddress(
    [
      Buffer.from("MerkleDistributor"),
      payer.toBuffer(),
    ],
    gumdropId
  );
}

async function getBubblegumNonce(): Promise<PublicKey> {
  const [nonce, _] = await PublicKey.findProgramAddress(
    [
      Buffer.from("bubblegum")
    ],
    BUBBLEGUM_PROGRAM_ID,
  );
  return nonce;
}

async function getBubblegumTreeAuthority(tree: PublicKey): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress(
    [
      tree.toBuffer(),
    ],
    BUBBLEGUM_PROGRAM_ID,
  ))[0];
}

type GumdropLeaf = {
  metadata: Metadata,
  publicKey: PublicKey,
};

type Creator = {
  creator: PublicKey,
  share: number,
};
type Metadata = {
  name: string,
  symbol: string,
  uri: string,
  sellerFeeBasisPoints: number,
  primarySaleHappened: boolean,
  isMutable: boolean,
  editionNonce: null,
  tokenStandard: null,
  tokenProgramVersion: {
    original: {},
  },
  collections: null,
  uses: null,
  creators: Creator[],
};

const METADATA = [
  {
    name: "A",
    symbol: "A",
    uri: "A",
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: {
      original: {},
    },
    collections: null,
    uses: null,
    creators: [],
  },
  {
    name: "B",
    symbol: "B",
    uri: "www.solana.com",
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: {
      original: {},
    },
    collections: null,
    uses: null,
    creators: [],
  },
  {
    name: "C",
    symbol: "C",
    uri: "www.solana.com",
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: {
      original: {},
    },
    collections: null,
    uses: null,
    creators: [],
  },
  {
    name: "D",
    symbol: "D",
    uri: "www.solana.com",
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: {
      original: {},
    },
    collections: null,
    uses: null,
    creators: [],
  },
  {
    name: "E",
    symbol: "E",
    uri: "www.solana.com",
    sellerFeeBasisPoints: 0,
    primarySaleHappened: false,
    isMutable: false,
    editionNonce: null,
    tokenStandard: null,
    tokenProgramVersion: {
      original: {},
    },
    collections: null,
    uses: null,
    creators: [],
  },
];

function serializeMetadata(metadata: Metadata): Buffer {
  let writer = new BinaryWriter();
  writer.writeString(metadata.name)
  writer.writeString(metadata.symbol)
  writer.writeString(metadata.uri)
  writer.writeU16(metadata.sellerFeeBasisPoints)
  writer.writeU8(Number(metadata.primarySaleHappened))
  writer.writeU8(Number(metadata.isMutable))
  // edition nonce
  writer.writeU8(0)
  // token standard
  writer.writeU8(0)
  // Collection
  writer.writeU8(0)
  // Uses
  writer.writeU8(0)
  // token program
  writer.writeU8(0)
  // no creators :)
  writer.writeU32(0);
  return Buffer.from(writer.toArray())
}

function hashLeaf(leaf: GumdropLeaf, index: number, bubblegumTree: PublicKey): Buffer {
  const metadata = serializeMetadata(leaf.metadata);
  console.log("Metadata:", metadata);
  return Buffer.concat([
    // index
    (new anchor.BN(index)).toBuffer("le", 8),
    // claimant secret
    leaf.publicKey.toBuffer(),
    bubblegumTree.toBuffer(),
    // amount
    (new anchor.BN(1)).toBuffer("le", 8),
    metadata
  ]);
}

function buildGumdropTree(bubblegumTree: PublicKey, claimer: PublicKey): MerkleTree {
  let leaves: Buffer[] = [];
  METADATA.forEach((metadata, index) => {
    leaves.push(hashLeaf({
      metadata,
      publicKey: claimer
    }, index, bubblegumTree));
  });
  return new MerkleTree(leaves);
}

function initBubblegumNonce(nonce: PublicKey, payer: PublicKey): TransactionInstruction {
  return new TransactionInstruction({
    keys: [
      {
        pubkey: nonce,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: payer,
        isSigner: true,
        isWritable: true,
      }, {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      }
    ],
    programId: BUBBLEGUM_PROGRAM_ID,
    data: Buffer.from(Uint8Array.from([64, 206, 214, 231, 20, 15, 231, 41]))
  });
}

describe('Airdropping compressed NFTs with Gumdrop', () => {
  const connection = new Connection("http://127.0.0.1:8899", { commitment: "confirmed" });
  const payer = Keypair.generate();

  it("Works for at least 5 NFTs", async () => {

    const sig = await connection.requestAirdrop(payer.publicKey, 5 * 1e9);
    await connection.confirmTransaction(sig);

    const wallet = new NodeWallet(payer);
    // Initialize program-wide nonce
    const nonce = await getBubblegumNonce();
    const initNonceIx = initBubblegumNonce(nonce, payer.publicKey);

    // Generate Merkle Slab Keypair
    const merkleRollKeypair = Keypair.generate();
    const maxDepth = 20;
    const maxBufferSize = 64;

    // This has to be done after the keypair is known
    const gumdropTree = buildGumdropTree(merkleRollKeypair.publicKey, payer.publicKey);
    console.log("Gumdrop 🌲 root:", new PublicKey(gumdropTree.getRoot()).toString());

    const gumdrop = anchor.workspace.Gumdrop as anchor.Program<Gumdrop>;
    // const gumdrop = await Program.at("gdrpGjVffourzkdDRrQmySw4aTHr8a3xmQzzxSwFD1a", provider);
    const allocAccountIx = await initMerkleTreeInstruction(maxDepth, maxBufferSize, connection, merkleRollKeypair.publicKey, payer.publicKey);
    const [distributor, distributorBump] = await getDistributor(payer.publicKey, gumdrop.programId);
    const bubblegumTreeAuthority = await getBubblegumTreeAuthority(merkleRollKeypair.publicKey);

    const temporal = payer.publicKey;
    const createDistributorIx = gumdrop.instruction.newDistributorCompressed(
      // @ts-ignore
      distributorBump,
      gumdropTree.getRoot(),
      temporal,
      maxDepth,
      maxBufferSize,
      {
        accounts: {
          base: payer.publicKey,
          distributor,
          payer: payer.publicKey,
          bubblegumTree: merkleRollKeypair.publicKey,
          bubblegumTreeAuthority,
          bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
          gummyrollProgram: GUMMYROLL_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        },
        signers: [
          payer,
        ]
      }
    );
    const tx = new Transaction().add(initNonceIx).add(allocAccountIx).add(createDistributorIx);
    tx.feePayer = payer.publicKey;
    tx.recentBlockhash = (await connection.getRecentBlockhash("confirmed")).blockhash;
    console.log(tx);
    const txId = await connection.sendTransaction(tx, [merkleRollKeypair, payer], {
      skipPreflight: true
    });
    await succeedOrThrow(txId, connection);
    console.log("Compressed tree init succeeded 😎");

    // Get nonce key for all compressed NFTs
    let index = 0;
    while (index < METADATA.length) {
      const nftMetadata = METADATA[index];
      const proof = gumdropTree.getProof(index);
      console.log("Verified proof:", gumdropTree.verifyProof(index, proof, gumdropTree.getRoot()));
      const leafHash = gumdropTree.layers[0][0].buffer;
      console.log("\nLeaf hash:", leafHash.slice(0, 32), "\n\n");

      const indexBuf = (new anchor.BN(index)).toBuffer("le", 8);
      const [claimCount, claimBump] = await PublicKey.findProgramAddress(
        [
          Buffer.from("ClaimCount"),
          indexBuf,
          distributor.toBuffer(),
        ],
        gumdrop.programId
      );

      const ix = gumdrop.instruction.claimBubblegum(
        claimBump,
        (new anchor.BN(index)),
        (new anchor.BN(1)),
        payer.publicKey,
        serializeMetadata(nftMetadata),
        Array.from(proof),
        {
          accounts: {
            distributor,
            claimCount,
            payer: payer.publicKey,
            temporal,
            nonce,
            bubblegumTreeAuthority,
            bubblegumTree: merkleRollKeypair.publicKey,
            bubblegumProgram: BUBBLEGUM_PROGRAM_ID,
            gummyrollProgram: GUMMYROLL_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          },
          signers: [
            payer
          ]
        }
      );
      const claimTx = new Transaction().add(ix);
      claimTx.feePayer = payer.publicKey;
      claimTx.recentBlockhash = (await connection.getRecentBlockhash("confirmed")).blockhash;
      const txId = await connection.sendTransaction(claimTx, [payer], {
        skipPreflight: true
      });
      await succeedOrThrow(txId, connection);
      console.log("Succesfully airdropped compressed NFT @ index:", index);
      index++;
    }
  });
});
