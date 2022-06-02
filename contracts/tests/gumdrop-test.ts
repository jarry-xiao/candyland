import { PublicKey, Connection, Keypair, Transaction, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import * as anchor from "@project-serum/anchor";
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';
import { Gumdrop } from "../target/types/gumdrop";
import { MerkleTree } from './gumdropTree';
import { BinaryWriter } from 'borsh'
import { getMerkleRollAccountSize } from './merkle-roll-serde';
import { succeedOrThrow } from './utils';

async function initMerkleTreeInstruction(
  maxDepth: number,
  maxBufferSize: number,
  connection: Connection,
  merkleRoll: PublicKey,
  payer: PublicKey,
  gummyrollProgramId: PublicKey,
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
    programId: gummyrollProgramId,
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

async function getBubblegumNonce(bubblegumProgramId: PublicKey): Promise<PublicKey> {
  const [nonce, _] = await PublicKey.findProgramAddress(
    [
      Buffer.from("bubblegum")
    ],
    bubblegumProgramId,
  );
  return nonce;
}

async function getBubblegumTreeAuthority(tree: PublicKey, bubblegumProgramId: PublicKey): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress(
    [
      tree.toBuffer(),
    ],
    bubblegumProgramId,
  ))[0];
}

type GumdropLeaf = {
  metadata: TokenMetadata,
  publicKey: PublicKey,
};

type Creator = {
  creator: PublicKey,
  share: number,
};
type TokenMetadata = {
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

const TOKEN_METADATA = [
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

function serializeMetadata(metadata: TokenMetadata): Buffer {
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
  TOKEN_METADATA.forEach((metadata, index) => {
    leaves.push(hashLeaf({
      metadata,
      publicKey: claimer
    }, index, bubblegumTree));
  });
  return new MerkleTree(leaves);
}

function initBubblegumNonce(nonce: PublicKey, payer: PublicKey, bubblegumProgramId: PublicKey): TransactionInstruction {
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
    programId: bubblegumProgramId,
    data: Buffer.from(Uint8Array.from([64, 206, 214, 231, 20, 15, 231, 41]))
  });
}

describe('Airdropping compressed NFTs with Gumdrop', () => {
  const connection = new Connection("http://localhost:8899", { commitment: "confirmed" });
  const payer = Keypair.generate();
  const wallet = new NodeWallet(payer);
  anchor.setProvider(
    new anchor.Provider(
      connection,
      wallet,
      { commitment: "confirmed", skipPreflight: true })
  );
  const gumdrop = anchor.workspace.Gumdrop as anchor.Program<Gumdrop>;
  const BUBBLEGUM_PROGRAM_ID = anchor.workspace.Bubblegum.programId;
  const GUMMYROLL_PROGRAM_ID = anchor.workspace.Gummyroll.programId;

  const maxDepth = 20;
  const maxBufferSize = 64;
  let merkleRollKeypair: Keypair;
  let gumdropTree: MerkleTree;

  beforeEach(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 5 * 1e9);
    await connection.confirmTransaction(sig);
  });

  it("Works for at least 5 NFTs", async () => {
    // Generate Merkle Slab Keypair
    merkleRollKeypair = Keypair.generate();

    // This has to be done after the keypair is known
    gumdropTree = buildGumdropTree(merkleRollKeypair.publicKey, payer.publicKey);
    console.log("Gumdrop 🌲 root:", new PublicKey(gumdropTree.getRoot()).toString());

    // Initialize program-wide nonce
    const nonce = await getBubblegumNonce(BUBBLEGUM_PROGRAM_ID);
    const initNonceIx = initBubblegumNonce(nonce, payer.publicKey, BUBBLEGUM_PROGRAM_ID);

    // Init merkle tree
    const allocAccountIx = await initMerkleTreeInstruction(maxDepth, maxBufferSize, connection, merkleRollKeypair.publicKey, payer.publicKey, GUMMYROLL_PROGRAM_ID);
    const [distributor, distributorBump] = await getDistributor(payer.publicKey, gumdrop.programId);
    const bubblegumTreeAuthority = await getBubblegumTreeAuthority(merkleRollKeypair.publicKey, BUBBLEGUM_PROGRAM_ID);

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
    const tx = new Transaction()
      .add(initNonceIx)
      .add(allocAccountIx)
      .add(createDistributorIx);

    const txId = await gumdrop.provider.send(tx, [merkleRollKeypair, payer], {
      skipPreflight: true
    });
    await succeedOrThrow(txId, connection);
    console.log("Compressed tree init succeeded 😎");

    // Get nonce key for all compressed NFTs
    let index = 0;
    while (index < TOKEN_METADATA.length) {
      const nftMetadata = TOKEN_METADATA[index];
      const proof = gumdropTree.getProof(index);
      console.log("\nVerified proof:", gumdropTree.verifyProof(index, proof, gumdropTree.getRoot()));
      const leafHash = gumdropTree.layers[0][index].buffer;
      console.log("Leaf hash:", leafHash.slice(0, 32));

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
      console.log(`Succesfully airdropped compressed NFT @ index: ${index}\n`);
      index++;
    }
  });
});
