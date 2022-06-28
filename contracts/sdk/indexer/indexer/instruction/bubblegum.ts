import { NFTDatabaseConnection } from "../../db"
import { ParserState, OptionalInfo } from "../utils"
import { PublicKey, CompiledInstruction, CompiledInnerInstruction } from "@solana/web3.js"
import { BorshEventCoder, BorshInstructionCoder } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { ChangeLogEvent, ingestBubblegumCreateTree } from "../ingester";
import { decodeEvent } from "../utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../../utils";
import { Idl, IdlTypeDef } from '@project-serum/anchor/dist/cjs/idl';
import { IdlCoder } from '@project-serum/anchor/dist/cjs/coder/borsh/idl';
import { Layout } from "buffer-layout";


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
                await parseBubblegumCreateTree(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    innerInstructions
                )
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

function findWrapInstructions(accountKeys: PublicKey[], instructions: CompiledInstruction[]): CompiledInstruction[] {
    const wrapIxs = [];
    for (const ix of instructions) {
        if (accountKeys[ix.programIdIndex].equals(CANDY_WRAPPER_PROGRAM_ID)) {
            wrapIxs.push(ix);
        }
    }
    return wrapIxs;
}

function decodeEventInstructionData(
    idl: Idl,
    eventName: string,
    base58String: string,
) {
    const rawLayouts: [string, Layout<any>][] = idl.events.map((event) => {
        let eventTypeDef: IdlTypeDef = {
            name: event.name,
            type: {
                kind: "struct",
                fields: event.fields.map((f) => {
                    return { name: f.name, type: f.type };
                }),
            },
        };
        return [event.name, IdlCoder.typeDefLayout(eventTypeDef, idl.types)];
    });
    const layouts = new Map(rawLayouts);
    const buffer = bs58.decode(base58String);
    const layout = layouts.get(eventName);
    if (!layout) {
        console.error("Could not find corresponding layout for event:", eventName);
    }
    const data = layout.decode(buffer);
    return { data, name: eventName };
}

async function parseBubblegumCreateTree(
    db: NFTDatabaseConnection,
    slot: number,
    optionalInfo: OptionalInfo,
    parser: ParserState,
    accountKeys: PublicKey[],
    innerInstructions: CompiledInnerInstruction[],
) {
    let changeLogEvent: ChangeLogEvent | null = null;
    for (const innerInstruction of innerInstructions) {
        const wrapIxs = findWrapInstructions(accountKeys, innerInstruction.instructions);
        if (wrapIxs.length != 1) {
            console.error("Found too many or too little wrap inner instructions for bubblegum create tree instruction")
        }
        const eventData = decodeEventInstructionData(parser.Gummyroll.idl, "ChangeLogEvent", wrapIxs[0].data);
        changeLogEvent = eventData.data as ChangeLogEvent;
    }

    await ingestBubblegumCreateTree(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
    );
}
