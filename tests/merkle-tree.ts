import { BN } from "@project-serum/anchor";
import { keccak_256 } from "js-sha3";
import * as Collections from 'typescript-collections';

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
    id: number,
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

function emptyTreeNode(level: number, id: number): TreeNode {
    return {
        node: emptyNode(level),
        left: undefined,
        right: undefined,
        parent: undefined,
        level: level,
        id
    }
}

function buildLeaves(leaves: Buffer[]): [Collections.Queue<TreeNode>, TreeNode[]] {
    let nodes = new Collections.Queue<TreeNode>();
    let finalLeaves = [];
    leaves.forEach((buffer, index) => {
        const treeNode = {
            node: buffer,
            left: undefined,
            right: undefined,
            parent: undefined,
            level: 0,
            id: index
        };
        nodes.enqueue(treeNode);
        finalLeaves.push(treeNode)
    })
    return [nodes, finalLeaves];
}

/**
 * Initializes the tree from the array of leaves passed in
 */
export function buildTree(leaves: Buffer[]): Tree {
    let [nodes, finalLeaves] = buildLeaves(leaves);
    let seqNum = leaves.length;
    while (nodes.size() > 1) {
        let left = nodes.dequeue();
        const level = left.level;

        let right: TreeNode;
        if (level != nodes.peek().level) {
            right = emptyTreeNode(level, seqNum);
            seqNum++;
        } else {
            right = nodes.dequeue();
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
        nodes.enqueue(parent);    
    }

    return {
        root: nodes.peek().node,
        leaves: finalLeaves,
    }
}

/**
 * Takes a built Tree and returns the proof to leaf
 */
export function getProofOfLeaf(tree: Tree, idx: number): [TreeNode[], number] {
    let proof: TreeNode[] = [];

    let node: TreeNode;
    node = tree.leaves[idx];

    while (typeof node.parent !== 'undefined') {
        let parent = node.parent;
        if (parent.left.id === node.id) {
            proof.push(parent.right);
        } else {
            proof.push(parent.left);
        }
        node = node.parent;
    }

    return [proof, ~idx];
}

export function updateTree(tree: Tree, newNode: Buffer, index: number) {
    let leaf = tree.leaves[index];
    leaf.node = newNode;
    let node = leaf.parent;
    while (typeof node.parent !== 'undefined') {
        node.node = hash(node.left.node, node.right.node);
        node = node.parent;
    }
}

/**
 * Uses on-chain hash fn to hash together buffers
 */
export function hash(left: Buffer, right: Buffer): Buffer {
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
