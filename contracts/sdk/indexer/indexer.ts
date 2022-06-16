import { Keypair } from "@solana/web3.js";
import { Connection } from "@solana/web3.js";
import { PROGRAM_ID as BUBBLEGUM_PROGRAM_ID } from "../bubblegum/src/generated";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from "../gummyroll/index";
import * as anchor from "@project-serum/anchor";
import { Bubblegum } from "../../target/types/bubblegum";
import { Gummyroll } from "../../target/types/gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { loadProgram, handleLogs } from "./indexer/utils";
import { bootstrap } from "./db";

const localhostUrl = "http://127.0.0.1:8899";
let Bubblegum: anchor.Program<Bubblegum>;
let Gummyroll: anchor.Program<Gummyroll>;

async function main() {
  const endpoint = localhostUrl;
  const connection = new Connection(endpoint, "confirmed");
  const payer = Keypair.generate();
  const provider = new anchor.Provider(connection, new NodeWallet(payer), {
    commitment: "confirmed",
  });
  let db = await bootstrap();
  console.log("Finished bootstrapping DB");
  Gummyroll = loadProgram(
    provider,
    GUMMYROLL_PROGRAM_ID,
    "target/idl/gummyroll.json"
  ) as anchor.Program<Gummyroll>;
  Bubblegum = loadProgram(
    provider,
    BUBBLEGUM_PROGRAM_ID,
    "target/idl/bubblegum.json"
  ) as anchor.Program<Bubblegum>;
  console.log("loaded programs...");
  let subscriptionId = connection.onLogs(
    BUBBLEGUM_PROGRAM_ID,
    async (logs, ctx) =>
      await handleLogs(db, logs, ctx, { Gummyroll, Bubblegum })
  );
}

main();
