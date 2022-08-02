import { PublicKey } from "@solana/web3.js";
import * as borsh from "borsh";
import { BN } from "@project-serum/anchor";
import { readPublicKey } from "@sorend-solana/utils";
import { getMerkleRollAccountSize } from "../convenience";
import { PathNode } from "../generated";

/**
 * Manually create a model for MerkleRoll in order to deserialize correctly
 */
export class OnChainMerkleRoll {
  header: MerkleRollHeader;
  roll: MerkleRoll;

  constructor(header: MerkleRollHeader, roll: MerkleRoll) {
    this.header = header;
    this.roll = roll;
  }

  getChangeLogsWithNodeIndex(): PathNode[][] {
    const mask = this.header.maxBufferSize - 1;
    let pathNodeList: PathNode[][] = [];
    for (let j = 0; j < this.roll.bufferSize; j++) {
      let pathNodes: PathNode[] = [];
      let idx = (this.roll.activeIndex - j) & mask;
      let changeLog = this.roll.changeLogs[idx];
      let pathLen = changeLog.pathNodes.length;
      for (const [lvl, key] of changeLog.pathNodes.entries()) {
        let nodeIdx = (1 << (pathLen - lvl)) + (changeLog.index >> lvl);
        pathNodes.push({
          node: Array.from(key.toBuffer()),
          index: nodeIdx,
        });
      }
      pathNodes.push({
        node: Array.from(changeLog.root.toBuffer()),
        index: 1,
      });
      pathNodeList.push(pathNodes);
    }
    return pathNodeList;
  }
}

type MerkleRollHeader = {
  maxDepth: number; // u32
  maxBufferSize: number; // u32
  authority: PublicKey;
  creationSlot: BN;
};

type MerkleRoll = {
  sequenceNumber: BN; // u64
  activeIndex: number; // u64
  bufferSize: number; // u64
  changeLogs: ChangeLog[];
  rightMostPath: Path;
};

type ChangeLog = {
  root: PublicKey;
  pathNodes: PublicKey[];
  index: number; // u32
  _padding: number; // u32
};

type Path = {
  leaf: PublicKey;
  proof: PublicKey[];
  index: number;
  _padding: number;
};

export function decodeMerkleRoll(buffer: Buffer): OnChainMerkleRoll {
  let reader = new borsh.BinaryReader(buffer);

  let header: MerkleRollHeader = {
    maxBufferSize: reader.readU32(),
    maxDepth: reader.readU32(),
    authority: readPublicKey(reader),
    creationSlot: reader.readU64(),
  };

  // Decode MerkleRoll
  let sequenceNumber = reader.readU64();
  let activeIndex = reader.readU64().toNumber();
  let bufferSize = reader.readU64().toNumber();

  // Decode ChangeLogs
  let changeLogs: ChangeLog[] = [];
  for (let i = 0; i < header.maxBufferSize; i++) {
    let root = readPublicKey(reader);

    let pathNodes: PublicKey[] = [];
    for (let j = 0; j < header.maxDepth; j++) {
      pathNodes.push(readPublicKey(reader));
    }
    changeLogs.push({
      pathNodes,
      root,
      index: reader.readU32(),
      _padding: reader.readU32(),
    });
  }

  // Decode Right-Most Path
  let leaf = readPublicKey(reader);
  let proof: PublicKey[] = [];
  for (let j = 0; j < header.maxDepth; j++) {
    proof.push(readPublicKey(reader));
  }
  const rightMostPath = {
    proof,
    leaf,
    index: reader.readU32(),
    _padding: reader.readU32(),
  };

  const roll = {
    sequenceNumber,
    activeIndex,
    bufferSize,
    changeLogs,
    rightMostPath,
  };

  if (
    getMerkleRollAccountSize(header.maxDepth, header.maxBufferSize) !=
    reader.offset
  ) {
    throw new Error(
      "Failed to process whole buffer when deserializing Merkle Account Data"
    );
  }
  return new OnChainMerkleRoll(header, roll);
}
