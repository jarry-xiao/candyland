import sqlite3 from "sqlite3";
import { open, Database, Statement } from "sqlite";

export class NFTDatabaseConnection {
  connection: Database<sqlite3.Database, sqlite3.Statement>;
  getTreeStmt: Statement<sqlite3.Statement>;
  upsertStmt: Statement<sqlite3.Statement>;
  constructor(
    connection: Database<sqlite3.Database, sqlite3.Statement>,
    getTreeStmt: Statement<sqlite3.Statement>,
    upsertStmt: Statement<sqlite3.Statement>
  ) {
    this.connection = connection;
    this.getTreeStmt = getTreeStmt;
    this.upsertStmt = upsertStmt;
  }
}

// this is a top-level await
export async function bootstrap(): Promise<NFTDatabaseConnection> {
  // open the database
  const db = await open({
    filename: "/tmp/merkle.db",
    driver: sqlite3.Database,
  });

  await db.run(
    `CREATE TABLE IF NOT EXISTS merkle (
        node_idx INT,
        seq INT,
        level INT,
        hash TEXT,
        PRIMARY KEY (seq, node_idx)
    )`
  );
  const getTreeStmt = await db.prepare(
    `SELECT 
        DISTINCT node_idx, level, hash, MAX(seq) as seq
    FROM merkle
    group by node_idx`
  );
  const upsertStmt = await db.prepare(
    `INSERT INTO
    merkle(node_idx, seq, level, hash)
    VALUES
    (@node_idx, @seq, @level, @hash)
    ON CONFLICT(node_idx, seq)
    DO UPDATE SET
        level=excluded.level,
        hash=excluded.hash
    ;`
  );

  return {
    connection: db,
    getTreeStmt,
    upsertStmt,
  };
}

