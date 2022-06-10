import * as anchor from "@project-serum/anchor";
import { Provider, Program } from "@project-serum/anchor";
import { Myprog } from "../target/types/myprog";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";

describe("myprog", () => {
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

  const prog = anchor.workspace.Myprog as Program<Myprog>;
  console.log(prog);
  it("basic test", async () => {
      await prog.rpc.initialize({});
  });
});
