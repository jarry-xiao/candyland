import sqlite3 from "sqlite3";
import { open, Database, Statement } from "sqlite";
import { PathNode } from "../gummyroll";
import { PublicKey } from "@solana/web3.js";
import { keccak_256 } from "js-sha3";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";

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

  async upsert(rows: Array<[PathNode, number, number]>) {
    this.connection.db.serialize(() => {
      this.connection.run("BEGIN TRANSACTION");
      for (const [node, seq, i] of rows) {
        this.connection.run(
          `
            INSERT INTO 
            merkle(node_idx, seq, level, hash)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(node_idx)
            DO UPDATE SET
              level=excluded.level,
              hash=excluded.hash,
              seq=excluded.seq
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
      this.tree[row.node_idx] = [row.seq, row.hash];
    }
    return res;
  }

  async getProof(hash: Buffer): Promise<Proof | null> {
    let hashString = new PublicKey(hash).toBase58();
    let res = await this.connection.all(
      `
        SELECT node_idx
        FROM merkle
        WHERE hash = ? and level = 0
      `,
      hashString,
    );
    if (res.length == 1) {
      let nodeIdx = res[0].node_idx;
      let nodes = [];
      let n = nodeIdx;
      while (n > 1) {
        if (n % 2 == 0) {
          nodes.push(n + 1);
        } else {
          nodes.push(n - 1);
        }
        n /= 2;
      }
      nodes.push(1);
      res = await this.connection.all(
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
      return {
        leaf: hash,
        root: bs58.decode(root.hash),
        proofNodes: proof,
        index: leafIdx,
      };
    } else {
      console.log("Failed to find leaf hash");
      return null;
    }
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
  const db = await open({
    filename: "/tmp/merkle.db",
    driver: sqlite3.Database,
  });

  await db.run(
    `
      CREATE TABLE IF NOT EXISTS merkle (
        node_idx INT,
        seq INT,
        level INT,
        hash TEXT,
        PRIMARY KEY (node_idx)
      )
    `
  );

  return new NFTDatabaseConnection(db);
}
