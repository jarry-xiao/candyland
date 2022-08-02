import * as anchor from "@project-serum/anchor";
import { BN, AnchorProvider, Program } from "@project-serum/anchor";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import {
  Connection,
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
} from "@solana/web3.js";
import { assert } from "chai";
import { decodeMerkleRoll } from "@sorend-solana/gummyroll-solita";

export async function assertOnChainMerkleRollProperties(
  connection: Connection,
  expectedMaxDepth: number,
  expectedMaxBufferSize: number,
  expectedAuthority: PublicKey,
  expectedRoot: PublicKey,
  merkleRollPubkey: PublicKey
) {
  const merkleRoll = await connection.getAccountInfo(merkleRollPubkey);

  if (!merkleRoll) {
    throw new Error("Merkle Roll account data unexpectedly null!");
  }

  const merkleRollAcct = decodeMerkleRoll(merkleRoll.data);

  assert(
    merkleRollAcct.header.maxDepth === expectedMaxDepth,
    `Max depth does not match ${merkleRollAcct.header.maxDepth}, expected ${expectedMaxDepth}`
  );
  assert(
    merkleRollAcct.header.maxBufferSize === expectedMaxBufferSize,
    `Max buffer size does not match ${merkleRollAcct.header.maxBufferSize}, expected ${expectedMaxBufferSize}`
  );

  assert(
    merkleRollAcct.header.authority.equals(expectedAuthority),
    "Failed to write auth pubkey"
  );

  assert(
    merkleRollAcct.roll.changeLogs[0].root.equals(expectedRoot),
    "On chain root does not match root passed in instruction"
  );
}