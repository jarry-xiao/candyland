import { BN, Provider, Program } from "@project-serum/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Transaction,
  Connection as web3Connection,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  val,
  strToByteArray,
  strToByteUint8Array
} from "@sorend-solana/utils";

export function getBufferFromStringArr(stringArray: string[]): Buffer {
  const buffer = stringArray.reduce(
    (prevVal, curVal) =>
      Buffer.concat([prevVal, Buffer.from(curVal)]),
    Buffer.from([])
  );
  return buffer;
}

export function assertInRangeAndReturnNum(val: number, propertyName: string, lowerBound: number = 0, upperBound?: number): number {
  if (val < lowerBound) {
    throw new Error(`❌ ${propertyName} is too small (perhaps unexpectedly negative)! Double check your JSON init props. ❌`);
  }
  if (typeof upperBound != 'undefined' && val > upperBound) {
    throw new Error(`❌ ${propertyName} is too large! Double check your JSON init props. ❌`);
  }
  return val;
}

export function assertNonNegativeAndConvertToBN(val: number, propertyName: string): BN {
  if (val < 0) {
    throw new Error(`❌ Did not expect ${propertyName} to be negative! Double check your JSON init props. ❌`);
  }
  return new BN(val);
}

export function assertLengthAndConvertToPublicKey(s: string, propertyName: string): PublicKey {
  try {
    return new PublicKey(s);
  } catch(e) {
    throw new Error(`❌ ${propertyName} has an incorrect length to be a publickey! Publickeys are 32 bytes. Double check your JSON init props. ❌`);
  }
}

export function assertLengthAndConvertByteArray(s: string, size: number, propertyName: string): number[] {
  if (s.length >  size) {
    throw new Error(`❌ ${propertyName} has too many characters! Reference docs for size limits of GumballHeader props. Double check your JSON init props. ❌`);
  }
  return strToByteArray(s, size);
}

export function deserializeCreatorKeys(keys: string[], shares: number[]): PublicKey[] {
  if (keys.length != shares.length) {
    throw new Error(`❌ creatorKeys must be the same length as creatorShares ❌`);
  }
  if (keys.length > 4) {
    throw new Error(`❌ creatorKeys is too long! We currently only support at most 4 creators ❌`);
  } else {
    return keys.map((key, i) => assertLengthAndConvertToPublicKey(key, `Creator key ${i}`))
  }
}

export function deserializeCreatorShares(shares: number[]): Uint8Array {
  if (shares.length > 0 && shares.reduce((acc, share) => acc + share, 0) != 100) {
    throw new Error(`❌ creatorShares must sum to exactly 100% ❌`);
  }
  return Uint8Array.from(shares);
}