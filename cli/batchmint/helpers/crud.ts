import {
    PublicKey,
    Keypair,
    Transaction,
    SystemProgram
} from '@solana/web3.js';
import { hash, getProofOfAssetFromServer, checkProof } from '../../../contracts/tests/merkle-tree';
import { Gummyroll, IDL as GUMMYROLL_IDL } from '../../../target/types/gummyroll';
import { GummyrollCrud, IDL as GUMMYROLL_CRUD_IDL } from '../../../target/types/gummyroll_crud';
import log from 'loglevel';
import { Program, Provider } from '@project-serum/anchor';
import {
    PROGRAM_ID as GUMMYROLL_PROGRAM_ID,
    getMerkleRollAccountSize
} from "@sorend-solana/gummyroll-solita";
import { GUMMYROLL_CRUD_PROGRAM_ID } from '../../helpers/constants';
import { confirmTxOrThrow } from '../../helpers/utils';
import fetch from 'cross-fetch';
import { join } from 'path';
import { readFileSync, existsSync } from 'fs';

async function loadGummyroll(provider: Provider): Promise<Program<Gummyroll>> {
    // only on non-localnet
    // return await Program.at(GUMMYROLL_PROGRAM_ID, provider) as Program<Gummyroll>
    return new Program(GUMMYROLL_IDL, GUMMYROLL_PROGRAM_ID, provider);
}

async function loadGummyrollCrud(provider: Provider): Promise<Program<GummyrollCrud>> {
    // return await Program.at(GUMMYROLL_CRUD_PROGRAM_ID, provider) as Program<GummyrollCrud>
    return new Program(GUMMYROLL_CRUD_IDL, GUMMYROLL_CRUD_PROGRAM_ID, provider);
}

async function getTreeAuthorityPDA(
    gummyrollCrud: Program<GummyrollCrud>,
    treeAddress: PublicKey,
    treeAdmin: PublicKey
) {
    const seeds = [
        Buffer.from("gummyroll-crud-authority-pda", "utf-8"),
        treeAddress.toBuffer(),
        treeAdmin.toBuffer(),
    ];
    return await PublicKey.findProgramAddress(
        seeds,
        gummyrollCrud.programId
    );
}

type ProofInfo = {
    root: Buffer,
    leaf: Buffer,
    proof: PublicKey[],
    index: number
}

function loadProofInfo(fname: string): ProofInfo {
    const proofInfo = JSON.parse(readFileSync(fname).toString()) as ProofInfo
    return {
        root: new PublicKey(proofInfo.root).toBuffer(),
        leaf: new PublicKey(proofInfo.leaf).toBuffer(),
        proof: proofInfo.proof.map((nodeStr) => new PublicKey(nodeStr)),
        index: proofInfo.index
    }

}

export function loadBatchInfoFromDir(dir: string): {
    changeLogDbUri: string,
    metadataDbUri: string,
    proofInfo: ProofInfo
} {
    const uploadFname = join(dir, "upload.json");
    if (!existsSync(uploadFname)) {
        throw new Error("😢 No upload json found, cannot load changelog and metadata db uris")
    }
    const uploadInfo = JSON.parse(readFileSync(uploadFname).toString());

    return {
        metadataDbUri: uploadInfo['metadataUri'] as string,
        changeLogDbUri: uploadInfo['changelogUri'] as string,
        proofInfo: loadProofInfo(join(dir, "proof.json"))
    }
}

export async function batchInitTree(
    provider: Provider,
    treeAdminKeypair: Keypair,
    maxDepth: number,
    maxBufferSize: number,
    changeLogDbUri: string,
    metadataDbUri: string,
    proofInfo: ProofInfo
): Promise<PublicKey> {
    const treeKeypair = Keypair.generate();
    const requiredSpace = getMerkleRollAccountSize(maxDepth, maxBufferSize);

    const gummyroll = await loadGummyroll(provider);
    const gummyrollCrud = await loadGummyrollCrud(provider);

    const allocGummyrollAccountIx = SystemProgram.createAccount({
        fromPubkey: treeAdminKeypair.publicKey,
        newAccountPubkey: treeKeypair.publicKey,
        lamports:
            await gummyroll.provider.connection.getMinimumBalanceForRentExemption(
                requiredSpace
            ),
        space: requiredSpace,
        programId: gummyroll.programId,
    });

    const [treeAuthorityPDA] = await getTreeAuthorityPDA(
        gummyrollCrud,
        treeKeypair.publicKey,
        treeAdminKeypair.publicKey
    );

    const batchTreeIx = gummyrollCrud.instruction.createTreeWithRoot(
        maxDepth,
        maxBufferSize,
        proofInfo.root,
        proofInfo.leaf,
        proofInfo.index,
        Buffer.from(changeLogDbUri),
        Buffer.from(metadataDbUri),
        {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: treeAuthorityPDA,
                gummyrollProgram: gummyroll.programId,
                merkleRoll: treeKeypair.publicKey,
            },
            signers: [treeAdminKeypair],
            remainingAccounts: proofInfo.proof.map((pubkey) => {
                return {
                    pubkey,
                    isSigner: false,
                    isWritable: false
                }
            }),
        }
    );

    const tx = new Transaction().add(allocGummyrollAccountIx).add(batchTreeIx);
    const batchTreeTxId = await gummyroll.provider.send(
        tx,
        [treeAdminKeypair, treeKeypair],
        {
            commitment: "confirmed",
            skipPreflight: true,
        }
    );
    log.info("Sent batch init transaction:", batchTreeTxId);

    await confirmTxOrThrow(gummyroll.provider.connection, batchTreeTxId);
    return treeKeypair.publicKey;
}

export async function initEmptyTree(
    provider: Provider,
    treeAdminKeypair: Keypair,
    maxDepth: number,
    maxBufferSize: number
): Promise<PublicKey> {
    const treeKeypair = Keypair.generate();
    const requiredSpace = getMerkleRollAccountSize(maxDepth, maxBufferSize);

    const gummyroll = await loadGummyroll(provider);
    const gummyrollCrud = await loadGummyrollCrud(provider);

    const allocGummyrollAccountIx = SystemProgram.createAccount({
        fromPubkey: treeAdminKeypair.publicKey,
        newAccountPubkey: treeKeypair.publicKey,
        lamports:
            await gummyroll.provider.connection.getMinimumBalanceForRentExemption(
                requiredSpace
            ),
        space: requiredSpace,
        programId: gummyroll.programId,
    });

    const [treeAuthorityPDA] = await getTreeAuthorityPDA(
        gummyrollCrud,
        treeKeypair.publicKey,
        treeAdminKeypair.publicKey
    );

    const createTreeIx = gummyrollCrud.instruction.createTree(
        maxDepth,
        maxBufferSize,
        {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: treeAuthorityPDA,
                gummyrollProgram: gummyroll.programId,
                merkleRoll: treeKeypair.publicKey,
            },
            signers: [treeAdminKeypair],
        }
    );

    const tx = new Transaction().add(allocGummyrollAccountIx).add(createTreeIx);
    const createTreeTxId = await gummyroll.provider.send(
        tx,
        [treeAdminKeypair, treeKeypair],
        {
            commitment: "confirmed",
        }
    );
    log.info("Sent init empty transaction:", createTreeTxId);

    await confirmTxOrThrow(gummyroll.provider.connection, createTreeTxId);
    return treeKeypair.publicKey;
}

export async function appendMessage(
    provider: Provider,
    treeAdminKeypair: Keypair,
    treeAddress: PublicKey,
    message: string,
) {
    const gummyroll = await loadGummyroll(provider);
    const gummyrollCrud = await loadGummyrollCrud(provider);

    const [treeAuthorityPDA] = await getTreeAuthorityPDA(
        gummyrollCrud,
        treeAddress,
        treeAdminKeypair.publicKey
    );
    const signers = [treeAdminKeypair];
    const addIx = gummyrollCrud.instruction.add(Buffer.from(message), {
        accounts: {
            authority: treeAdminKeypair.publicKey,
            authorityPda: treeAuthorityPDA,
            gummyrollProgram: gummyroll.programId,
            merkleRoll: treeAddress,
        },
        signers,
    });

    const appendTxId = await gummyrollCrud.provider.send(new Transaction().add(addIx), signers, {
        commitment: "confirmed",
    });
    log.info("Sent append message transaction:", appendTxId);
    await confirmTxOrThrow(gummyroll.provider.connection, appendTxId);
}

export async function showProof(
    proofUrl: string,
    treeAddress: PublicKey,
    index: number,
) {
    const proofInfo = await getProofOfAssetFromServer(proofUrl, treeAddress, index);
    const root = new PublicKey(proofInfo.root).toString();
    const hash = new PublicKey(proofInfo.hash).toString();
    console.log(`Proof found for leaf at ${index} in tree ${treeAddress.toString()}`)
    console.log(`Root: ${root}`);
    console.log(`Current leaf hash: ${hash}`);
    console.log(`Proof:`);
    proofInfo.proof.map((node, index) => {
        console.log(`${index}: ${new PublicKey(node).toString()}`)
    });
}

type Asset = {
    data: string,
    index: number,
    owner: string,
    treeAccount: string,
    treeAdmin: string,
    hash: string,
}

function logAsset(asset: Asset) {
    log.info(`"${asset.data}`);
    log.info(`  ${asset.index}: ${asset.hash}`);
    log.info(`  Tree: ${asset.treeAccount}`);
    log.info(`  Tree admin: ${asset.treeAdmin}`);
    log.info(`  Asset owner: ${asset.owner}`);
}

async function getAssetsFromServer(proofUrl: string, owner: string): Promise<Asset[]> {
    const response = await fetch(`${proofUrl}/owner/${owner}/assets`, { method: "GET" }).then(resp => resp.json())
    return response.data as Asset[]
}

export async function showAssets(
    proofUrl: string,
    owner: string,
) {
    const assets = await getAssetsFromServer(proofUrl, owner);
    log.info("Found assets:")
    log.info("--------------")
    assets.map((asset) => logAsset(asset));
}

export async function removeMessage(
    provider: Provider,
    proofUrl: string,
    treeAdminKeypair: Keypair,
    treeAddress: PublicKey,
    index: number,
    owner: PublicKey,
    message: string,
) {
    const gummyroll = await loadGummyroll(provider);
    const gummyrollCrud = await loadGummyrollCrud(provider);

    const proofInfo = await getProofOfAssetFromServer(proofUrl, treeAddress, index);
    const root = new PublicKey(proofInfo.root).toBuffer();
    const leafHash = getLeafHash(owner, message);

    if (new PublicKey(leafHash).toString() !== new PublicKey(proofInfo.hash).toString()) {
        console.log("Expected:", new PublicKey(proofInfo.hash).toString());
        console.log("Calculated:", new PublicKey(leafHash).toString());
        throw new Error("❌ Leaf message does not match what's in tree! ❌");
    }

    const nodeProof = proofInfo.proof.map((node) => ({
        pubkey: new PublicKey(node),
        isSigner: false,
        isWritable: false,
    }));
    const [treeAuthorityPDA] = await getTreeAuthorityPDA(
        gummyrollCrud,
        treeAddress,
        treeAdminKeypair.publicKey
    );
    const signers = [treeAdminKeypair];
    const removeIx = gummyrollCrud.instruction.remove(
        Array.from(root),
        Array.from(leafHash),
        index,
        {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: treeAuthorityPDA,
                gummyrollProgram: gummyroll.programId,
                merkleRoll: treeAddress,
            },
            signers,
            remainingAccounts: nodeProof,
        }
    );
    const tx = new Transaction().add(removeIx);
    const removeTxId = await gummyrollCrud.provider.send(tx, signers, {
        commitment: "confirmed",
        skipPreflight: true,
    });

    log.info("Sent remove message transaction:", removeTxId);
    confirmTxOrThrow(gummyroll.provider.connection, removeTxId);
}

export async function transferMessageOwner(
    provider: Provider,
    proofUrl: string,
    treeAdminKeypair: Keypair,
    treeAddress: PublicKey,
    index: number,
    owner: PublicKey,
    newOwner: PublicKey,
    message: string,
) {
    const gummyroll = await loadGummyroll(provider);
    const gummyrollCrud = await loadGummyrollCrud(provider);

    const proofInfo = await getProofOfAssetFromServer(proofUrl, treeAddress, index);

    if (!checkProof(index, proofInfo.root, proofInfo.hash, proofInfo.proof)) {
        throw new Error("Hash did not match!")
    }

    const root = new PublicKey(proofInfo.root).toBuffer();
    const leafHash = getLeafHash(owner, message);

    if (new PublicKey(leafHash).toString() !== new PublicKey(proofInfo.hash).toString()) {
        console.log("Expected:", new PublicKey(proofInfo.hash).toString());
        console.log("Calculated:", new PublicKey(leafHash).toString());
        throw new Error("❌ This tx will fail, since the owner + message combo does not match what's in tree! ❌");
    }

    const nodeProof = proofInfo.proof.map((node) => ({
        pubkey: new PublicKey(node),
        isSigner: false,
        isWritable: false,
    }));

    const [treeAuthorityPDA] = await getTreeAuthorityPDA(
        gummyrollCrud,
        treeAddress,
        treeAdminKeypair.publicKey
    );
    const signers = [treeAdminKeypair];

    log.info("Submitting transfer");
    log.info(`${owner.toString()} -> ${newOwner.toString()}: "${message}"`)
    const transferIx = gummyrollCrud.instruction.transfer(
        Array.from(root),
        Buffer.from(message),
        index,
        {
            accounts: {
                authority: treeAdminKeypair.publicKey,
                authorityPda: treeAuthorityPDA,
                gummyrollProgram: gummyroll.programId,
                merkleRoll: treeAddress,
                newOwner: newOwner,
                owner: owner,
            },
            signers,
            remainingAccounts: nodeProof,
        }
    );
    const tx = new Transaction().add(transferIx);
    const transferTxId = await gummyrollCrud.provider.send(tx, signers, {
        commitment: "confirmed",
        skipPreflight: true,
    });

    log.info("Sent transfer message transaction:", transferTxId);
    confirmTxOrThrow(gummyroll.provider.connection, transferTxId);
}

function getLeafHash(owner: PublicKey | undefined, message: string) {
    return hash(owner?.toBuffer() ?? Buffer.alloc(32), Buffer.from(message));
}
