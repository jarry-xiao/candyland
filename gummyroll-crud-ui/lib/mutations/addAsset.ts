import GummyrollIdl from "../../../target/idl/gummyroll.json";
import * as anchor from "@project-serum/anchor";
import getGummyrollCrudProgram from "../anchor_programs/getGummyrollCrudProgram";
import getGummyrollCrudAuthorityPDA from "../anchor_programs/pdas/getGummyrollCrudAuthorityPDA";
import { Gummyroll } from "../../../target/types/gummyroll";
import { IdlEvent } from "@project-serum/anchor/dist/cjs/idl";
import GummyrollProgramId from "../anchor_programs/GummyrollProgramId";

type GetChangeLogEvent<T extends IdlEvent> = T["name"] extends "ChangeLogEvent"
  ? T
  : never;
type ChangeLogEvent = GetChangeLogEvent<Gummyroll["events"][number]>;

export default async function addAsset(
  treeAccount: anchor.web3.PublicKey,
  treeAdmin: anchor.web3.PublicKey,
  data: string
) {
  const program = getGummyrollCrudProgram();
  const [authorityPda] = await getGummyrollCrudAuthorityPDA(
    treeAccount,
    treeAdmin
  );
  const txid = await program.methods
    .add(Buffer.from(data, "utf-8"))
    .accounts({
      authority: treeAdmin,
      authorityPda,
      merkleRoll: treeAccount,
      gummyrollProgram: GummyrollProgramId,
    })
    .rpc({ commitment: "confirmed" });
  const transaction = await program.provider.connection.getTransaction(txid, {
    commitment: "confirmed",
  });
  try {
    const eventParser = new anchor.EventParser(
      GummyrollProgramId,
      new anchor.BorshCoder(GummyrollIdl as anchor.Idl)
    );
    let foundEventData: anchor.Event<ChangeLogEvent>["data"] | null = null;
    eventParser.parseLogs(transaction!.meta!.logMessages!, (log) => {
      if (foundEventData) {
        return;
      }
      if (log.name === "ChangeLogEvent") {
        foundEventData = (log as anchor.Event<ChangeLogEvent>).data;
      }
    });
    if (foundEventData == null) {
      throw new Error("Could not find index of new asset");
    }
    return (foundEventData as anchor.Event<ChangeLogEvent>["data"]).index;
  } catch (e) {
    console.error(e);
    throw new Error("Could not find index of new asset");
  }
}
