import { BN } from "@project-serum/anchor";
import { TransactionInstruction, PublicKey, Connection, AccountInfo } from "@solana/web3.js";
import { Nonce, PROGRAM_ID, Voucher } from './generated';

export async function getBubblegumAuthorityPDA(merkleRollPubKey: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
        [merkleRollPubKey.toBuffer()],
        PROGRAM_ID
    );
    return bubblegumAuthorityPDAKey;
}

export async function getNonceCount(connection: Connection, tree: PublicKey): Promise<BN> {
    const treeAuthority = await getBubblegumAuthorityPDA(tree);
    return new BN((await Nonce.fromAccountAddress(connection, treeAuthority)).count);
}

export async function getVoucherPDA(connection: Connection, tree: PublicKey, leafIndex: number): Promise<PublicKey> {
    let [voucher] = await PublicKey.findProgramAddress(
        [
            Buffer.from("voucher", "utf8"),
            tree.toBuffer(),
            new BN(leafIndex).toBuffer("le", 8),
        ],
        PROGRAM_ID
    );
    return voucher;
}
