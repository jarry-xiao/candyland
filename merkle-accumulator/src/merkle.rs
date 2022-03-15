use solana_program::keccak::hashv;
use std::collections::VecDeque;
use std::rc::Rc;
use std::cell::{RefCell, RefMut, Ref};

pub type Node = [u8; 32];
pub const MAX_SIZE: usize = 64;
pub const MAX_DEPTH: usize = 20;
pub const PADDING: usize = 12;
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
    leaf_nodes: Vec<Rc<RefCell<TreeNode>>>,
    free_list: VecDeque<Rc<RefCell<TreeNode>>>,
    root: Node,
}

#[derive(Clone)]
struct TreeNode {
    node: Node,
    left: Option<Rc<RefCell<TreeNode>>>,
    right: Option<Rc<RefCell<TreeNode>>>,
    parent: Option<Rc<RefCell<TreeNode>>>,
    level: u32,
}

struct ProofNode {
    node: Node,
    is_right: bool,
}

impl TreeNode {
    pub fn new(
        node: Node,
        left: Rc<RefCell<TreeNode>>,
        right: Rc<RefCell<TreeNode>>,
        level: u32,
    ) -> Self {
        Self {
            node,
            left: Some(left),
            right: Some(right),
            parent: None,
            level,
        }
    }


    pub fn new_empty(level: u32) -> Self {
        Self {
            node: empty_node(level),
            left: None,
            right: None,
            parent: None,
            level: level,
        }
    }

    pub fn assign_parent(node: &mut Rc<RefCell<TreeNode>>, parent: Rc<RefCell<TreeNode>>) {
        node.borrow_mut().parent = Some(parent);
    }
}


fn empty_node(level: u32) -> Node {
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
    // pub fn build_root(self: &Self) -> Node {
    //     let mut current = [0;32];
    //     let mut initialized= false;
    //     let mut left = true;

    //     let mut tree = Vec::<HashingNode>::new();

    //     for tree_node in self.leaf_nodes.iter() {
    //         if !initialized {
    //             current = tree_node
    //         } else {
    //             current.left = current.r
    //         } //     }
    //     return [0; 32];
    // }

    pub fn build_root(&mut self) {
        let mut tree = VecDeque::<Rc<RefCell<TreeNode>>>::new();
        for node in self.leaf_nodes.iter() {
            tree.push_back(node.clone());
        }

        while tree.len() > 1 {
            let left = tree.pop_front().unwrap();
            let level = left.borrow().level;
            let mut right = if level != tree[0].borrow().level {
                Rc::new(RefCell::new(TreeNode::new_empty(left.borrow().level)))
            } else {
                tree.pop_front().unwrap()
            };
            let mut hashed_parent = [0; 32];

            hashed_parent.copy_from_slice(hashv(&[&left.borrow().node, &right.borrow().node]).as_ref());
            let parent = Rc::new(RefCell::new(TreeNode::new(hashed_parent, left.clone(), right.clone(), level + 1)));
            TreeNode::assign_parent(&mut left, parent.clone());
            TreeNode::assign_parent(&mut right, parent.clone());
            tree.push_back(parent);
        }

        self.root = tree[0].borrow().node;
    }

    pub fn get_proof(self: Self, idx: usize) -> Vec<ProofNode> {
        let proof_vec = Vec::<ProofNode>::new();
        let mut node = self.leaf_nodes[idx];
        let mut parent = node.borrow().parent;
        while node.borrow().parent.is_some() {
            let current_node: &Ref<TreeNode> = node.borrow();
            let parent_node = node.borrow().parent.unwrap().clone();
            // if node.borrow().left.borrow().unwrap().node == child_key {
            //     proof_vec.push(ProofNode {
            //         node: node.right.unwrap().node,
            //         is_right: true,
            //     });
            // } else {
            //     proof_vec.push(ProofNode {
            //         node: node.left.unwrap().node,
            //         is_right: false,
            //     });
            // }
        }
        proof_vec
    }

    pub fn verify_proof(self, proof: Vec<ProofNode>, mut leaf: Node) -> bool {
        for node in proof.iter() {
            if node.is_right {
                let res = hashv(&[&leaf, &node.node]);
                leaf.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[&node.node, &leaf]);
                leaf.copy_from_slice(res.as_ref());
            }
        }
        leaf == self.root
    }

    pub fn add_leaf(&mut self, leaf: Node, idx: usize) {
        self.leaf_nodes[idx].borrow_mut().node = leaf;
        self.build_root();
    }

    pub fn remove_leaf(&mut self, leaf: Node, idx: usize) {
        self.leaf_nodes[idx].borrow_mut().node = [0; 32];
        self.build_root();
    }

    // pub fn verify_proof(self: &Self, proof: Vec<Node>, path: u32, leaf: Node) -> bool {
    //     let key = recompute(leaf, proof.as_slice(), path);
    //     key == self.root
    // }
}
