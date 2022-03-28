import { Provider } from "@project-serum/anchor";

export async function logTx(provider: Provider, txId: string) {
  await provider.connection.confirmTransaction(txId, "confirmed");
  console.log(
    (await provider.connection.getConfirmedTransaction(txId, "confirmed")).meta
      .logMessages
  );
};

