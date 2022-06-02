import { Connection } from '@solana/web3.js';
import { Provider } from "@project-serum/anchor";
import { TransactionInstruction, Transaction, Signer } from "@solana/web3.js";

export async function logTx(provider: Provider, txId: string, verbose: boolean = true) {
  await provider.connection.confirmTransaction(txId, "confirmed");
  if (verbose) {
    console.log(
      (await provider.connection.getConfirmedTransaction(txId, "confirmed")).meta
        .logMessages
    );
  }
};

export async function execute(
  provider: Provider,
  instructions: TransactionInstruction[],
  signers: Signer[],
  skipPreflight: boolean = false
): Promise<String> {
  let tx = new Transaction();
  instructions.map((ix) => { tx = tx.add(ix) });
  const txid = await provider.send(tx, signers, {
    commitment: "confirmed",
    skipPreflight,
  });
  await logTx(provider, txid, false);
  return txid;
}

export async function succeedOrThrow(txId: string, connection: Connection) {
  const err = (await connection.confirmTransaction(txId, "confirmed")).value.err
  if (err) {
    throw new Error(`${txId} failed: \n${JSON.stringify(err)}\n`);
  }
}
