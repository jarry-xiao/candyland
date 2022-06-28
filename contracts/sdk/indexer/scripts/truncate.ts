import { PublicKey, Keypair, Connection, Transaction } from "@solana/web3.js";
import * as anchor from '@project-serum/anchor';
import { ParserState, handleLogsAtomic, loadProgram, loadPrograms } from "../indexer/utils";
import { createMintV1Instruction, createCreateTreeInstruction } from '../../bubblegum/src/generated/instructions';
import { createAllocTreeIx } from "../../gummyroll";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { CANDY_WRAPPER_PROGRAM_ID } from "../../utils";
import { Key, TokenStandard } from "@metaplex-foundation/mpl-token-metadata";
import { getBubblegumAuthorityPDA } from "../../bubblegum/src/convenience";
import { execute } from "../../../tests/utils";
import { getCreateTreeIxs } from "../../bubblegum/src/convenience";
import { PROGRAM_ID as GUMMYROLL_PROGRAM_ID } from '../../gummyroll';
import { TokenProgramVersion } from "../../bubblegum/src/generated";

// const url = "http://api.explorer.mainnet-beta.solana.com";
const url = "http://127.0.0.1:8899";

function keypairFromString(seed: string) {
    const spaces = "                                         ";
    const buffer = Buffer.from(`${seed}${spaces}`.slice(0, 32));;
    return Keypair.fromSeed(Uint8Array.from(buffer));
}

const MAX_BUFFER_SIZE = 256;
const MAX_DEPTH = 20;
const CANOPY_DEPTH = 5;

/**
 * Truncates logs by sending too many append instructions
 * This forces the indexer to go into gap-filling mode
 * and use the WRAP CPI args to complete the database.
 */
async function main() {
    const endpoint = url;
    const connection = new Connection(endpoint, "confirmed");
    const payer = keypairFromString('bubblegum-mini-milady');
    const provider = new anchor.Provider(connection, new NodeWallet(payer), {
        commitment: "confirmed",
    });

    const { txId, tx } = await pureAppend(connection, provider, payer);

    if (tx.meta.logMessages) {
        let logsTruncated = false;
        for (const log of tx.meta.logMessages) {
            // console.log(log);
            if (log.startsWith('Log truncated')) {
                logsTruncated = true;
            }
        }
        console.log(`Logs truncated: ${logsTruncated}`);
    }
}

async function pureAppend(
    connection: Connection,
    provider: anchor.Provider,
    payer: Keypair,
) {
    const bgumTree = keypairFromString("bubblegum-mini-tree");
    const authority = await getBubblegumAuthorityPDA(bgumTree.publicKey);

    const acctInfo = await connection.getAccountInfo(bgumTree.publicKey, "confirmed");
    if (!acctInfo || acctInfo.lamports === 0) {
        console.log("Creating tree:", bgumTree.publicKey.toBase58());
        console.log("Requesting airdrop:", await connection.requestAirdrop(payer.publicKey, 5e10));
        const ixs = await getCreateTreeIxs(connection, MAX_DEPTH, MAX_BUFFER_SIZE, CANOPY_DEPTH, payer.publicKey, bgumTree.publicKey, payer.publicKey);
        console.log("Created bubblegum tree with txId:", await execute(provider, ixs, [payer, bgumTree], true));
    } else {
        console.log("Bubblegum tree already exists:", bgumTree.publicKey.toBase58());
    }

    const mintIxs = [];
    for (let i = 0; i < 6; i++) {
        mintIxs.push(createMintV1Instruction(
            {
                owner: payer.publicKey,
                delegate: payer.publicKey,
                authority,
                candyWrapper: CANDY_WRAPPER_PROGRAM_ID,
                gummyrollProgram: GUMMYROLL_PROGRAM_ID,
                mintAuthority: payer.publicKey,
                merkleSlab: bgumTree.publicKey,
            },
            {
                message: {
                    name: `${i}`,
                    symbol: `MILADY`,
                    uri: "www.remilia.org",
                    sellerFeeBasisPoints: 0,
                    primarySaleHappened: false,
                    isMutable: false,
                    uses: null,
                    collection: null,
                    creators: [],
                    tokenProgramVersion: TokenProgramVersion.Original,
                    tokenStandard: TokenStandard.NonFungible,
                    editionNonce: 0,
                }
            }
        ));
    }
    console.log("Sending barrel roll of transactions");
    // now do a barrel roll
    const txId = await execute(provider, mintIxs, [payer], true);
    console.log(`Barrel roll of mint transactions here: ${txId}`);
    const tx = await connection.getTransaction(txId, { commitment: 'confirmed' });
    return { txId, tx };
}

// Two things here:
// - create tree
// - pure appends stuck in a tx
// Or
// - create gumball machine
// - dispense as many as possible

main();
