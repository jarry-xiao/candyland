import { TransactionInstruction, PublicKey, Connection } from "@solana/web3.js";
import { PROGRAM_ID } from './generated';

export async function getBubblegumAuthorityPDA(merkleRollPubKey: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
        [merkleRollPubKey.toBuffer()],
        PROGRAM_ID
    );
    return bubblegumAuthorityPDAKey;
}
