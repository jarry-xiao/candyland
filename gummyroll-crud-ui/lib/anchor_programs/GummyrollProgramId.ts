import * as anchor from "@project-serum/anchor";
import GummyrollIdl from "../../../target/idl/gummyroll.json";

export default new anchor.web3.PublicKey(
  GummyrollIdl.metadata?.address ??
    "GRoLLzvxpxxu2PGNJMMeZPyMxjAUH9pKqxGXV9DGiceU"
);
