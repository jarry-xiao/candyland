import { BN } from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import { BothPartiesNeedToAgreeToSaleError } from "../deps/metaplex-program-library/auction-house/js/src/generated";
import { TreasuryIsNotEmptyError } from "../deps/metaplex-program-library/fixed-price-sale/js/src";

const MAX_DEPTH = 20;
let CACHE_EMPTY_NODE = new Map<number, Buffer>();

type Tree = {
    leaves: TreeNode[]
    root: Buffer,
}

type TreeNode = {
    node: Buffer,
    left: TreeNode | undefined,
    right: TreeNode | undefined,
    parent: TreeNode | undefined,
    level: number,
    id: BN,
}

const generateLeafNode = (seeds) => {
  let leaf = Buffer.alloc(32);
  for (const seed of seeds) {
    leaf = Buffer.from(keccak_256.digest([...leaf, ...seed]));
  }
  return leaf;
};


function emptyNode(level: number): Buffer {
    if (CACHE_EMPTY_NODE.has(level)) {
        return CACHE_EMPTY_NODE.get(level);
    }
    if (level == 0) {
        return Buffer.alloc(32)
    }

    let result = hash(emptyNode(level - 1), emptyNode(level - 1));
    CACHE_EMPTY_NODE.set(level, result);
    return result;
}

function emptyTreeNode(level: number, id: BN): TreeNode {
    return {
        node: emptyNode(level),
        left: undefined,
        right: undefined,
        parent: undefined,
        level: level,
        id
    }
}

function buildLeaves(leaves: Buffer[]): TreeNode[] {
    let nodes = [];
    leaves.forEach((buffer, index) => {
        nodes.push({
            node: buffer,
            left: undefined,
            right: undefined,
            parent: undefined,
            level: 0,
            id: new BN(index)
        })
    })
    return nodes;
}

/**
 * Initializes the tree from the array of leaves passed in
 */
export function buildTree(leaves: Buffer[]): Tree {
    const initialLeaves = buildLeaves(leaves);
    let nodes = initialLeaves;
    let seqNum = new BN(leaves.length);
    while (nodes.length > 1) {
        let left = nodes.pop();
        const level = left.level;

        let right: TreeNode;
        if (level != nodes[0].level) {
            right = emptyTreeNode(level, seqNum);
            seqNum = seqNum.add(new BN(1));
        } else {
            right = nodes.pop();
        }

        let parent: TreeNode = {
            node: hash(left.node, right.node),
            left: left,
            right: right,
            parent: undefined,
            level: level + 1,
            id: seqNum
        }
        left.parent = parent;
        right.parent = parent;
        nodes.push(parent)
        
    }

    return {
        root: nodes[0].node,
        leaves: initialLeaves,
    }
}

/**
 * Takes a built Tree and returns the proof to leaf
 */
function getProofOfLeaf(tree: Tree, idx: BN) {
    let node: TreeNode;
    if (idx.gt(new BN(tree.leaves.length - 1))) {
        node = emptyTreeNode(0, idx);
    } else {
        node = tree.leaves[idx.toNumber()]
    }

    while (typeof node.parent != 'undefined') {

    }
}

function hash(left: Buffer, right: Buffer): Buffer {
    return Buffer.from(keccak_256.digest(Buffer.concat([left, right])));
}


/**
 *  Does not build tree, just returns root of tree from leaves
 */
function hashLeaves(leaves: Buffer[]): Buffer {
    let nodes = leaves;
    let level = 0;
    while (level < MAX_DEPTH) {
        let next_nodes = [];

        if (nodes.length == 0) {
            nodes = [emptyNode(level), emptyNode(level)];
        }

        while (nodes.length > 0) {
            let left = nodes.pop();
            let right: Buffer;

            if (nodes.length > 0) {
                right = nodes.pop();
            } else {
                right = emptyNode(level);
            }
            next_nodes.push(hash(left, right));
        }

        level++;
        nodes = next_nodes
    }
    return nodes[0];
}
