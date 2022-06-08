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

  async getProof(hash: Buffer): Promise<Proof | null> {
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
      return this.generateProof(nodeIdx, hash);
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
  const db = await open({
    filename: "/tmp/merkle.db",
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
      )
    `
  );

  return new NFTDatabaseConnection(db);
}
