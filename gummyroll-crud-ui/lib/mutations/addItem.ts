import GummyrollCrudIdl from "../../../target/idl/gummyroll_crud.json";
import * as anchor from "@project-serum/anchor";
import { Idl, Program, web3 } from "@project-serum/anchor";

// @ts-ignore
let program: Program<Idl>;

const feePayer = anchor.web3.Keypair.generate();

const wallet = {
  async signTransaction(tx: web3.Transaction): Promise<web3.Transaction> {
    tx.partialSign(feePayer);
    return tx;
  },
  async signAllTransactions(): Promise<web3.Transaction[]> {
    return null as unknown as web3.Transaction[];
  },
  publicKey: feePayer.publicKey,
};

anchor.setProvider(
  new anchor.Provider(
    new anchor.web3.Connection("http://localhost:8899"),
    wallet,
    anchor.Provider.defaultOptions()
  )
);
anchor.getProvider().connection.requestAirdrop(feePayer.publicKey, 2e9);

export default async function addItem(
  treeAccount: anchor.web3.PublicKey,
  data: string
) {
  if (program == null) {
    // @ts-ignore
    program = new Program<typeof GummyrollCrudIdl>(
      GummyrollCrudIdl as Idl,
      process.env.NEXT_PUBLIC_GUMMYROLL_CRUD_PROGRAM_ID!
    );
  }
  const txid = await program.methods
    .add(Buffer.from(data))
    .accounts({
      merkleRoll: treeAccount,
      owner: feePayer.publicKey,
      gummyrollProgram: process.env.NEXT_PUBLIC_GUMMYROLL_PROGRAM_ID!,
    })
    .signers([feePayer])
    .rpc({ commitment: "confirmed" });
  const transaction = await program.provider.connection.getTransaction(txid, {
    commitment: "confirmed",
  });
  const index = 0; // TODO scan the logs for the index of the newly inserted item.
  return index;
}
