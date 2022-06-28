import { NFTDatabaseConnection } from "../../db"
import { ParserState, OptionalInfo } from "../utils"
import { PublicKey, CompiledInstruction, CompiledInnerInstruction } from "@solana/web3.js"
import { BorshInstructionCoder } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";

/// Copied from https://github.com/solana-labs/solana/blob/d07b0798504f757340868d15c199aba9bd00ba5d/explorer/src/utils/anchor.tsx#L57
export async function parseBubblegumInstruction(
    db: NFTDatabaseConnection,
    slot: number,
    parser: ParserState,
    optionalInfo: OptionalInfo,
    accountKeys: PublicKey[],
    instruction: CompiledInstruction,
    innerInstructions: CompiledInnerInstruction[],
) {
    const coder = new BorshInstructionCoder(parser.Bubblegum.idl);
    const decodedIx = coder.decode(bs58.decode(instruction.data));
    if (decodedIx) {
        const name = decodedIx.name.charAt(0).toUpperCase() + decodedIx.name.slice(1);
        console.log(`Found: ${name}`);
        switch (name) {
            case "CreateTree":
                // await parseBubblegumCreateTree(
                //   db,
                //   parsedLog.logs,
                //   slot,
                //   parser,
                //   optionalInfo
                // );
                break;
            case "MintV1":
                // await parseBubblegumMint(db, parsedLog.logs, slot, parser, optionalInfo);
                break;
            case "Redeem":
                // await parseReplaceLeaf(
                //   db,
                //   parsedLog.logs,
                //   slot,
                //   parser,
                //   optionalInfo,
                //   false
                // );
                break;
            case "CancelRedeem":
                // await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
                break;
            case "Burn":
                // await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
                break;
            case "Transfer":
                // await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
                break;
            case "Delegate":
                // await parseReplaceLeaf(db, parsedLog.logs, slot, parser, optionalInfo);
                break;
        }
    } else {
        console.error("Could not decode Bubblegum found in slot:", slot);
    }
}
