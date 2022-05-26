import { Program } from '@project-serum/anchor';
import { Gummyroll } from "../../../contracts/target/types/gummyroll";
import { Keypair, PublicKey, TransactionInstruction } from '@solana/web3.js';

export function createReplaceIx(
    gummyroll: Program<Gummyroll>,
    authority: Keypair,
    merkleRoll: PublicKey,
    treeRoot: Buffer,
    previousLeaf: Buffer,
    newLeaf: Buffer,
    index: number,
    proof: Buffer[]
): TransactionInstruction {
    const nodeProof = proof.map((node) => {
        return {
            pubkey: new PublicKey(node),
            isSigner: false,
            isWritable: false,
        };
    });
    return gummyroll.instruction.replaceLeaf(
        { inner: Array.from(treeRoot) },
        { inner: Array.from(previousLeaf) },
        { inner: Array.from(newLeaf) },
        index,
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
            },
            signers: [authority],
            remainingAccounts: nodeProof,
        }
    );
}

export function createAppendIx(
    gummyroll: Program<Gummyroll>,
    newLeaf: Buffer | ArrayLike<number>,
    authority: Keypair,
    appendAuthority: Keypair,
    merkleRoll: PublicKey,
): TransactionInstruction {
    return gummyroll.instruction.append(
        { inner: Array.from(newLeaf) },
        {
            accounts: {
                merkleRoll,
                authority: authority.publicKey,
                appendAuthority: appendAuthority.publicKey,
            },
            signers: [authority, appendAuthority],
        }
    );
}
