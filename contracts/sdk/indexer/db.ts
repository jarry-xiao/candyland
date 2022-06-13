import sqlite3 from "sqlite3";
import { open, Database, Statement } from "sqlite";
import { PathNode } from "../gummyroll";
import { PublicKey } from "@solana/web3.js";
import { keccak_256 } from "js-sha3";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { NewLeafEvent } from "./indexer/bubblegum";
import { BN } from "@project-serum/anchor";
import { Beet, bignum } from "@metaplex-foundation/beet";
import {
  LeafSchema,
  redeemInstructionDiscriminator,
} from "../bubblegum/src/generated";
import { ChangeLogEvent } from "./indexer/gummyroll";
let fs = require("fs");

/**
 * Uses on-chain hash fn to hash together buffers
 */
export function hash(left: Buffer, right: Buffer): Buffer {
  return Buffer.from(keccak_256.digest(Buffer.concat([left, right])));
}

export class NFTDatabaseConnection {
  connection: Database<sqlite3.Database, sqlite3.Statement>;
  tree: Map<number, [number, string]>;
  emptyNodeCache: Map<number, Buffer>;

  constructor(connection: Database<sqlite3.Database, sqlite3.Statement>) {
    this.connection = connection;
    this.tree = new Map<number, [number, string]>();
    this.emptyNodeCache = new Map<number, Buffer>();
  }

  async beginTransaction() {
    return this.connection.run("BEGIN TRANSACTION");
  }

  async rollback() {
    return this.connection.run("ROLLBACK");
  }

  async commit() {
    return this.connection.run("COMMIT");
  }

  async upsert(rows: Array<[PathNode, number, number]>) {
    this.connection.db.serialize(() => {
      this.connection.run("BEGIN TRANSACTION");
      for (const [node, seq, i] of rows) {
        this.connection.run(
          `
            INSERT INTO 
            merkle(node_idx, seq, level, hash)
            VALUES (?, ?, ?, ?)
          `,
          node.index,
          seq,
          i,
          node.node.toBase58()
        );
      }
      this.connection.run("COMMIT");
    });
  }

  async updateChangeLogs(changeLog: ChangeLogEvent) {
    console.log("Update Change Log");
    if (changeLog.seq == 0) {
      return;
    }
    for (const [i, pathNode] of changeLog.path.entries()) {
      this.connection.run(
        `
          INSERT INTO 
          merkle(node_idx, seq, level, hash)
          VALUES (?, ?, ?, ?)
        `,
        pathNode.index,
        changeLog.seq,
        i,
        new PublicKey(pathNode.node).toBase58()
      );
    }
  }

  async updateLeafSchema(leafSchema: LeafSchema, leafHash: PublicKey) {
    console.log("Update Leaf Schema");
    this.connection.run(
      `
        INSERT INTO
        leaf_schema(
          nonce,
          owner,
          delegate,
          data_hash,
          creator_hash,
          leaf_hash  
        )
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT (nonce)
        DO UPDATE SET 
          owner = excluded.owner,
          delegate = excluded.delegate,
          data_hash = excluded.data_hash,
          creator_hash = excluded.creator_hash,
          leaf_hash = excluded.leaf_hash
      `,
      (leafSchema.nonce.valueOf() as BN).toNumber(),
      leafSchema.owner.toBase58(),
      leafSchema.delegate.toBase58(),
      bs58.encode(leafSchema.dataHash),
      bs58.encode(leafSchema.creatorHash),
      leafHash.toBase58()
    );
  }

  async updateNFTMetadata(newLeafEvent: NewLeafEvent, nonce: bignum) {
    console.log("Update NFT");
    const uri = newLeafEvent.metadata.uri;
    const name = newLeafEvent.metadata.name;
    const symbol = newLeafEvent.metadata.symbol;
    const primarySaleHappened = newLeafEvent.metadata.primarySaleHappened;
    const sellerFeeBasisPoints = newLeafEvent.metadata.sellerFeeBasisPoints;
    const isMutable = newLeafEvent.metadata.isMutable;
    const creators = newLeafEvent.metadata.creators;
    this.connection.run(
      `
        INSERT INTO 
        nft(
          nonce,
          uri,
          name,
          symbol,
          primary_sale_happened,
          seller_fee_basis_points,
          is_mutable
        )
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (nonce)
        DO UPDATE SET
          uri = excluded.uri,
          name = excluded.name,
          symbol = excluded.symbol,
          primary_sale_happened = excluded.primary_sale_happened,
          seller_fee_basis_points = excluded.seller_fee_basis_points,
          is_mutable = excluded.is_mutable
      `,
      (nonce as BN).toNumber(),
      uri,
      name,
      symbol,
      primarySaleHappened,
      sellerFeeBasisPoints,
      isMutable
    );
    for (const creator of creators) {
      this.connection.run(
        `
            INSERT INTO 
            creators(
              nonce,
              creator,
              share,
              verified 
            )
            VALUES (?, ?, ?, ?)
          `,
        nonce,
        creator.address,
        creator.share,
        creator.verified
      );
    }
  }

  emptyNode(level: number): Buffer {
    if (this.emptyNodeCache.has(level)) {
      return this.emptyNodeCache.get(level);
    }
    if (level == 0) {
      return Buffer.alloc(32);
    }
    let result = hash(this.emptyNode(level - 1), this.emptyNode(level - 1));
    this.emptyNodeCache.set(level, result);
    return result;
  }

  async updateTree() {
    let res = await this.connection.all(
      `
        SELECT DISTINCT 
        node_idx, hash, level, max(seq) as seq
        FROM merkle
        GROUP BY node_idx
      `
    );
    for (const row of res) {
      this.tree.set(row.node_idx, [row.seq, row.hash]);
    }
    return res;
  }

  async getSequenceNumbers() {
    return new Set<number>(
      (
        await this.connection.all(
          `
            SELECT DISTINCT seq 
            FROM merkle
            ORDER by seq
          `
        )
      ).map((x) => x.seq)
    );
  }

  async getAllLeaves() {
    let leaves = await this.connection.all(
      `
        SELECT DISTINCT node_idx, hash, max(seq) as seq
        FROM merkle
        WHERE level = 0
        GROUP BY node_idx
        ORDER BY node_idx
      `
    );
    let leafHashes = new Set<string>();
    if (leaves.length > 0) {
      for (const l of leaves) {
        leafHashes.add(l.hash);
      }
    }
    return leafHashes;
  }

  async getLeafIndices(): Promise<Array<[number, Buffer]>> {
    let leaves = await this.connection.all(
      `
        SELECT DISTINCT node_idx, hash, max(seq) as seq
        FROM merkle
        WHERE level = 0
        GROUP BY node_idx
        ORDER BY node_idx
      `
    );
    let leafIdxs = [];
    if (leaves.length > 0) {
      for (const l of leaves) {
        leafIdxs.push([l.node_idx, bs58.decode(l.hash)]);
      }
    }
    return leafIdxs;
  }

  async getProof(hash: Buffer, check: boolean = true): Promise<Proof | null> {
    let hashString = bs58.encode(hash);
    let res = await this.connection.all(
      `
        SELECT DISTINCT node_idx, max(seq) as seq
        FROM merkle
        WHERE hash = ? and level = 0
        GROUP BY node_idx
      `,
      hashString
    );
    if (res.length == 1) {
      let nodeIdx = res[0].node_idx;
      return this.generateProof(nodeIdx, hash, check);
    } else {
      return null;
    }
  }

  async generateProof(
    nodeIdx: number,
    hash: Buffer,
    check: boolean = true
  ): Promise<Proof | null> {
    let nodes = [];
    let n = nodeIdx;
    while (n > 1) {
      if (n % 2 == 0) {
        nodes.push(n + 1);
      } else {
        nodes.push(n - 1);
      }
      n >>= 1;
    }
    nodes.push(1);
    let res = await this.connection.all(
      `
      SELECT DISTINCT node_idx, hash, level, max(seq) as seq
      FROM merkle where node_idx in (${nodes.join(",")})
      GROUP BY node_idx
      ORDER BY level
      `
    );
    if (res.length < 1) {
      return null;
    }
    let root = res.pop();
    if (root.node_idx != 1) {
      return null;
    }
    let proof = [];
    for (let i = 0; i < root.level; i++) {
      proof.push(this.emptyNode(i));
    }
    for (const node of res) {
      proof[node.level] = bs58.decode(node.hash);
    }
    let leafIdx = nodeIdx - (1 << root.level);
    let inferredProof = {
      leaf: hash,
      root: bs58.decode(root.hash),
      proofNodes: proof,
      index: leafIdx,
    };
    if (check && !this.verifyProof(inferredProof)) {
      console.log("Proof is invalid");
      return null;
    }
    return inferredProof;
  }

  verifyProof(proof: Proof) {
    let node = proof.leaf;
    let index = proof.index;
    for (const [i, pNode] of proof.proofNodes.entries()) {
      if ((index >> i) % 2 === 0) {
        node = hash(node, new PublicKey(pNode).toBuffer());
      } else {
        node = hash(new PublicKey(pNode).toBuffer(), node);
      }
    }
    const rehashed = new PublicKey(node).toString();
    const received = new PublicKey(proof.root).toString();
    return rehashed === received;
  }
}

export type Proof = {
  root: Buffer;
  leaf: Buffer;
  proofNodes: Buffer[];
  index: number;
};

// this is a top-level await
export async function bootstrap(): Promise<NFTDatabaseConnection> {
  // open the database
  const dir = "db";
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir);
  }
  const db = await open({
    filename: `${dir}/merkle.db`,
    driver: sqlite3.Database,
  });

  await db.run(
    `
      CREATE TABLE IF NOT EXISTS merkle (
        id INTEGER PRIMARY KEY,
        node_idx INT,
        seq INT,
        level INT,
        hash TEXT
      );
    `
  );

  await db.run(
    `
    CREATE TABLE IF NOT EXISTS nft (
      nonce BIGINT PRIMARY KEY,
      name TEXT,
      symbol TEXT,
      uri TEXT,
      seller_fee_basis_points INT, 
      primary_sale_happened BOOLEAN, 
      is_mutable BOOLEAN
    );
    `
  );
  await db.run(
    `
    CREATE TABLE IF NOT EXISTS leaf_schema (
      nonce BIGINT PRIMARY KEY,
      owner TEXT,
      delegate TEXT,
      data_hash TEXT,
      creator_hash TEXT,
      leaf_hash TEXT
    );
    `
  );
  await db.run(
    `
    CREATE TABLE IF NOT EXISTS creators (
      nonce BIGINT,
      creator TEXT,
      share INT,
      verifed BOOLEAN 
    );
    `
  );

  return new NFTDatabaseConnection(db);
}
