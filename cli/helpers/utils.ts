import log from 'loglevel';
import * as fs from 'fs';
import { PublicKey, Keypair, Connection } from '@solana/web3.js';
import { Program, AnchorProvider } from '@project-serum/anchor';
import NodeWallet from '@project-serum/anchor/dist/cjs/nodewallet';

export async function getProvider(endpoint: string, payer: Keypair) {
    const connection = new Connection(endpoint);
    const provider = new AnchorProvider(
        connection,
        new NodeWallet(payer),
        {
            commitment: "confirmed",
            skipPreflight: true,
        }
    )
    // await connection.getTransaction(
    //     await connection.requestAirdrop(payer.publicKey, 1e9),
    //     { commitment: 'confirmed' },
    // )
    return provider;
}

export function loadWalletKey(keypair: string): Keypair {
    if (!keypair || keypair == '') {
        throw new Error('Keypair is required!');
    }
    keypair = keypair.replace("~", process.env.HOME);
    const loaded = Keypair.fromSecretKey(
        new Uint8Array(JSON.parse(fs.readFileSync(keypair).toString())),
    );
    return loaded;
}

export async function confirmTxOrThrow(connection: Connection, txId: string) {
    const result = await connection.confirmTransaction(txId, "confirmed");
    if (result.value.err) {
        throw new Error(`Failed to execute transaction: ${result.value.err.toString()}`);
    }
}
