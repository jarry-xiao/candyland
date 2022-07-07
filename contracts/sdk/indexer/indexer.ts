import { Keypair, Logs, Connection, Context } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../bubblegum/src/generated";
import * as anchor from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { handleLogsAtomic } from "./indexer/log/bubblegum";
import { loadPrograms, ParseResult, ParserState } from "./indexer/utils";
import { bootstrap, NFTDatabaseConnection } from "./db";
import { fetchAndPlugGaps, plugGapsFromSlot, validateTree } from "./backfiller";
const { program } = require("commander");

async function handleLogSubscription(
  connection: Connection,
  db: NFTDatabaseConnection,
  logs: Logs,
  ctx: Context,
  parserState: ParserState
) {
  const result = handleLogsAtomic(db, logs, ctx, parserState);
  if (result === ParseResult.LogTruncated) {
    console.log("\t\tLOG TRUNCATED\n\n\n\n");
    await plugGapsFromSlot(
      connection,
      db,
      parserState,
      ctx.slot,
      0,
      Number.MAX_SAFE_INTEGER
    );
  }
}

async function main(url: string, dbPath: string) {
  const endpoint = url;
  const connection = new Connection(endpoint, "confirmed");
  const payer = Keypair.generate();
  const provider = new anchor.Provider(connection, new NodeWallet(payer), {
    commitment: "confirmed",
  });
  let db = await bootstrap(dbPath);
  console.log("Finished bootstrapping DB");
  const parserState = loadPrograms(provider);
  console.log("loaded programs...");
  let subscriptionId = connection.onLogs(
    BUBBLEGUM_PROGRAM_ID,
    (logs, ctx) =>
      handleLogSubscription(connection, db, logs, ctx, parserState),
    "confirmed"
  );
  while (true) {
    try {
      const trees = await db.getTrees();
      for (const [treeId, depth] of trees) {
        console.log("Scanning for gaps");
        let maxSeq = await fetchAndPlugGaps(
          connection,
          db,
          0,
          treeId,
          parserState,
          5
        );
        console.log("Validation:");
        console.log(
          `    Off-chain tree ${treeId} is consistent: ${await validateTree(
            db,
            depth,
            treeId,
            0
          )}`
        );
        console.log("Moving to next tree");
      }
    } catch (e) {
      console.log("ERROR");
      console.log(e);
      continue;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
}

program.option("-u, --url <string>").option("-d, --db-path <string>");

program.parse(process.argv);
const options = program.opts();

let url = "http://127.0.0.1:8899";
if (options.url) {
  switch (options.url) {
    case "m":
      url = "https://api.mainnet-beta.solana.com";
      break;
    case "d":
      url = "https://api.mainnet-beta.solana.com";
      break;
    case "l":
      break;
    default:
      url = options.url;
      break;
  }
}
let dbPath = "db";
if (options.dbPath) {
  dbPath = options.dbPath;
}

main(url, dbPath);
