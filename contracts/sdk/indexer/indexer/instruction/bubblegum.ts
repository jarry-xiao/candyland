import { hash, NFTDatabaseConnection } from "../../db"
import { ParserState, OptionalInfo } from "../utils"
import { PublicKey, CompiledInstruction, CompiledInnerInstruction } from "@solana/web3.js"
import { BorshEventCoder, BorshInstructionCoder } from "@project-serum/anchor";
import { bs58 } from "@project-serum/anchor/dist/cjs/utils/bytes";
import { ChangeLogEvent, ingestBubblegumCreateTree, ingestBubblegumMint, LeafSchemaEvent, NewLeafEvent } from "../ingester";
import { decodeEvent } from "../utils";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../../utils";
import { Idl, IdlTypeDef } from '@project-serum/anchor/dist/cjs/idl';
import { IdlCoder } from '@project-serum/anchor/dist/cjs/coder/borsh/idl';
import { Layout } from "buffer-layout";
import { Creator, LeafSchema, MetadataArgs, metadataArgsBeet, TokenProgramVersion, TokenStandard } from "../../../bubblegum/src/generated";
import { keccak_256 } from "js-sha3";
import { getLeafAssetId } from "../../../bubblegum/src/convenience";
import * as beetSolana from '@metaplex-foundation/beet-solana'
import * as beet from '@metaplex-foundation/beet'

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
                parseBubblegumMint(
                    db,
                    slot,
                    optionalInfo,
                    parser,
                    accountKeys,
                    instruction,
                    innerInstructions
                )
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
    // console.log(layouts);
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

function digest(input: Buffer): Buffer {
    return Buffer.from(keccak_256.digest(input))
}

function destructureMintAccounts(
    accountKeys: PublicKey[],
    instruction: CompiledInstruction
) {
    return {
        owner: accountKeys[instruction.accounts[4]],
        delegate: accountKeys[instruction.accounts[5]],
        merkleSlab: accountKeys[instruction.accounts[6]],
    }
}

function getTokenProgramVersion(object: Object): TokenProgramVersion {
    if (Object.keys(object).includes("original")) {
        return TokenProgramVersion.Original
    } else if (Object.keys(object).includes("token2022")) {
        return TokenProgramVersion.Token2022
    } else {
        return object as TokenProgramVersion;
    }
}

function getTokenStandard(object: Object): TokenStandard {
    const keys = Object.keys(object);
    if (keys.includes("nonFungible")) {
        return TokenStandard.NonFungible
    } else if (keys.includes("fungible")) {
        return TokenStandard.Fungible
    } else if (keys.includes("fungibleAsset")) {
        return TokenStandard.FungibleAsset
    } else if (keys.includes("nonFungibleEdition")) {
        return TokenStandard.NonFungibleEdition
    } else {
        return object as TokenStandard;
    }
}

export function hashMetadata(message: MetadataArgs) {
    // Todo: fix Solita - This is an issue with beet serializing complex enums
    message.tokenStandard = getTokenStandard(message.tokenStandard);
    message.tokenProgramVersion = getTokenProgramVersion(message.tokenProgramVersion);

    const [serialized, byteSize] = metadataArgsBeet.serialize(message);
    if (byteSize < 20) {
        console.log(serialized.length);
        console.error("Unable to serialize metadata args properly")
    }
    return digest(serialized)
}

type UnverifiedCreator = {
    address: PublicKey,
    share: number
};

export const unverifiedCreatorBeet = new beet.BeetArgsStruct<UnverifiedCreator>(
    [
        ['address', beetSolana.publicKey],
        ['share', beet.u8],
    ],
    'UnverifiedCreator'
)

export function hashCreators(creators: Creator[]) {
    const bytes = [];
    for (const creator of creators) {
        const unverifiedCreator = {
            address: creator.address,
            share: creator.share
        }
        const [buffer, _byteSize] = unverifiedCreatorBeet.serialize(unverifiedCreator);
        bytes.push(buffer);
    }
    return digest(Buffer.concat(bytes));
}

async function leafSchemaFromLeafData(
    owner: PublicKey,
    delegate: PublicKey,
    treeId: PublicKey,
    newLeafData: NewLeafEvent
): Promise<LeafSchemaEvent> {
    const id = await getLeafAssetId(treeId, newLeafData.nonce);
    return {
        schema: {
            v1: {
                id,
                owner,
                delegate,
                dataHash: [...hashMetadata(newLeafData.metadata)],
                creatorHash: [...hashCreators(newLeafData.metadata.creators)],
                nonce: newLeafData.nonce,
            }
        }
    }
}

async function parseBubblegumMint(
    db: NFTDatabaseConnection,
    slot: number,
    optionalInfo: OptionalInfo,
    parser: ParserState,
    accountKeys: PublicKey[],
    instruction: CompiledInstruction,
    innerInstructions: CompiledInnerInstruction[],
) {
    let newLeafData: NewLeafEvent;
    let changeLogEvent: ChangeLogEvent;
    for (const innerInstruction of innerInstructions) {
        const wrapIxs = findWrapInstructions(accountKeys, innerInstruction.instructions);
        if (wrapIxs.length != 2) {
            console.error("Found too many or too little wrap inner instructions for bubblegum mint instruction")
        }
        newLeafData = decodeEventInstructionData(parser.Bubblegum.idl, "NewNFTEvent", wrapIxs[0].data).data as NewLeafEvent;
        changeLogEvent = decodeEventInstructionData(parser.Gummyroll.idl, "ChangeLogEvent", wrapIxs[1].data).data as ChangeLogEvent;
    }

    const { owner, delegate, merkleSlab } = destructureMintAccounts(accountKeys, instruction);
    const leafSchema = await leafSchemaFromLeafData(owner, delegate, merkleSlab, newLeafData);

    await ingestBubblegumMint(
        db,
        slot,
        optionalInfo,
        changeLogEvent,
        newLeafData,
        leafSchema,
    )
}
