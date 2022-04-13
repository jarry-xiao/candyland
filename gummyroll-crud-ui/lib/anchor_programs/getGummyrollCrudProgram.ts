import GummyrollCrudIdl from "../../../target/idl/gummyroll_crud.json";
import { GummyrollCrud } from "../../../target/types/gummyroll_crud";
import { Program } from "@project-serum/anchor";

let program: Program<GummyrollCrud>;

export default function getGummyrollCrudProgram() {
  if (program == null) {
    program = new Program<GummyrollCrud>(
      GummyrollCrudIdl as unknown as GummyrollCrud,
      GummyrollCrudIdl.metadata?.address ??
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
    );
  }
  return program;
}
