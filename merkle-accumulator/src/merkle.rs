use solana_program::keccak::hashv;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::VecDeque;
use std::iter::FromIterator;
use std::rc::Rc;

pub type Node = [u8; 32];
pub const MAX_SIZE: usize = 64;
pub const MAX_DEPTH: usize = 10;
pub const PADDING: usize = 32 - MAX_DEPTH; 
pub const MASK: u64 = MAX_SIZE as u64 - 1;

const MAX_LEAVES: u64 = 0x1 << MAX_DEPTH;

pub fn recompute(mut start: Node, path: &[Node], address: u32) -> Node {
    for (ix, s) in path.iter().enumerate() {
        if address >> ix & 1 == 1 {
            let res = hashv(&[&start, s.as_ref()]);
            start.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), &start]);
            start.copy_from_slice(res.as_ref());
        }
    }
    start
}

// Off-chain implentation to keep track of nodes
pub struct MerkleTree {
    pub leaf_nodes: Vec<Rc<RefCell<TreeNode>>>,
    pub free_list: VecDeque<Rc<RefCell<TreeNode>>>,
    pub root: Node,
    pub seq_num: u128,
}

impl MerkleTree {
    pub fn new(leaves: Vec<Node>) -> Self {
        let mut leaf_nodes = vec![];
        for (i, node) in leaves.iter().enumerate() {
            let mut tree_node = TreeNode::new_empty(0, i as u128);
            tree_node.node = node.clone();
            leaf_nodes.push(Rc::new(RefCell::new(tree_node)));
        }
        let (root, seq_num) = MerkleTree::build_root(&leaf_nodes);
        Self {
            leaf_nodes,
            free_list: VecDeque::new(),
            root,
            seq_num,
        }
    }
}

#[derive(Clone)]
pub struct TreeNode {
    node: Node,
    left: Option<Rc<RefCell<TreeNode>>>,
    right: Option<Rc<RefCell<TreeNode>>>,
    parent: Option<Rc<RefCell<TreeNode>>>,
    level: u32,
    id: u128,
}

pub struct ProofNode {
    node: Node,
    is_right: bool,
}

impl TreeNode {
    pub fn new(
        node: Node,
        left: Rc<RefCell<TreeNode>>,
        right: Rc<RefCell<TreeNode>>,
        level: u32,
        id: u128,
    ) -> Self {
        Self {
            node,
            left: Some(left),
            right: Some(right),
            parent: None,
            level,
            id,
        }
    }

    pub fn new_empty(level: u32, id: u128) -> Self {
        Self {
            node: empty_node(level),
            left: None,
            right: None,
            parent: None,
            level: level,
            id 
        }
    }

    pub fn assign_parent(node: &mut Rc<RefCell<TreeNode>>, parent: Rc<RefCell<TreeNode>>) {
        node.borrow_mut().parent = Some(parent);
    }
}

pub fn empty_node(level: u32) -> Node {
    let mut data = [0; 32];
    if level != 0 {
        let lower_empty = empty_node(level - 1);
        let hash = hashv(&[&lower_empty, &lower_empty]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}

impl MerkleTree {
    /// Builds root from stack of leaves
    pub fn build_root(leaves: &Vec<Rc<RefCell<TreeNode>>>) -> (Node, u128) {
        let mut tree = VecDeque::from_iter(leaves.iter().map(|x| Rc::clone(x)));
        let mut seq_num = leaves.len() as u128;
        while tree.len() > 1 {
            let mut left = tree.pop_front().unwrap();
            let level = left.borrow().level;
            let mut right = if level != tree[0].borrow().level {
                let node = Rc::new(RefCell::new(TreeNode::new_empty(level, seq_num)));
                seq_num += 1;
                node
            } else {
                tree.pop_front().unwrap()
            };
            let mut hashed_parent = [0; 32];

            hashed_parent
                .copy_from_slice(hashv(&[&left.borrow().node, &right.borrow().node]).as_ref());
            let parent = Rc::new(RefCell::new(TreeNode::new(
                hashed_parent,
                left.clone(),
                right.clone(),
                level + 1,
                seq_num
            )));
            TreeNode::assign_parent(&mut left, parent.clone());
            TreeNode::assign_parent(&mut right, parent.clone());
            tree.push_back(parent);
            seq_num += 1;
        }

        let root = tree[0].borrow().node.clone();
        (root, seq_num)
    }

    pub fn get_proof(&self, idx: usize) -> (Vec<Node>, u32) {
        let mut proof_vec = Vec::<ProofNode>::new();
        let mut node = Rc::clone(&self.leaf_nodes[idx]);
        loop {
            let ref_node = Rc::clone(&node);
            if ref_node.borrow().parent.is_none() {
                break;
            }
            let parent = Rc::clone(&ref_node.borrow().parent.as_ref().unwrap());
            if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id {
                proof_vec.push(ProofNode {
                    node: parent.borrow().right.as_ref().unwrap().borrow().node,
                    is_right: true,
                });
            } else {
                proof_vec.push(ProofNode {
                    node: parent.borrow().left.as_ref().unwrap().borrow().node,
                    is_right: false,
                });
            }
            node = parent;
        }
        let proof: Vec<Node> = proof_vec.iter().map(|x| x.node).collect();
        let mut path = 0;
        for p in proof_vec.iter().rev() {
            path <<= 1;
            path |= (p.is_right) as u32;
        }
        (proof, path)
    }

    fn update_path(&mut self, idx: usize) {
        let mut node = Rc::clone(&self.leaf_nodes[idx]);
        loop {
            let ref_node = Rc::clone(&node);
            if ref_node.borrow().parent.is_none() {
                self.root = ref_node.borrow().node;
                break;
            }
            let parent = Rc::clone(&ref_node.borrow().parent.as_ref().unwrap());
            let hash =
                if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id {
                    hashv(&[
                        &ref_node.borrow().node,
                        &parent.borrow().right.as_ref().unwrap().borrow().node,
                    ])
                } else {
                    hashv(&[
                        &parent.borrow().left.as_ref().unwrap().borrow().node,
                        &ref_node.borrow().node,
                    ])
                };
            node = parent;
            node.borrow_mut().node.copy_from_slice(hash.as_ref());
        }
    }

    pub fn get_node(&self, idx: usize) -> Node {
        self.leaf_nodes[idx].borrow().node
    }

    pub fn add_leaf(&mut self, leaf: Node, idx: usize) {
        self.leaf_nodes[idx].borrow_mut().node = leaf;
        self.update_path(idx)
    }

    pub fn remove_leaf(&mut self, idx: usize) {
        self.leaf_nodes[idx].borrow_mut().node = [0; 32];
        self.update_path(idx)
    }
}
