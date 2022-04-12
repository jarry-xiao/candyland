import * as anchor from "@project-serum/anchor";
import GummyrollIdl from "../../../target/idl/gummyroll.json";

export default new anchor.web3.PublicKey(
  GummyrollIdl.metadata?.address ??
    "GRoLLMza82AiYN7W9S9KCCtCyyPRAQP2ifBy4v4D5RMD"
);
