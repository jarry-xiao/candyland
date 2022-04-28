use anchor_lang::{prelude::*, solana_program::keccak};
use gummyroll::state::node::Node;

#[event]
pub struct LeafSchemaEvent {
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data_hash: [u8; 32],
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Default, Debug)]
pub struct LeafSchema {
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data_hash: [u8; 32],
}

impl LeafSchema {
    pub fn new(owner: Pubkey, delegate: Pubkey, nonce: u128, data_hash: [u8; 32]) -> Self {
        Self {
            owner,
            delegate,
            nonce,
            data_hash,
        }
    }

    pub fn to_event(&self) -> LeafSchemaEvent {
        LeafSchemaEvent {
            owner: self.owner,
            delegate: self.delegate,
            nonce: self.nonce,
            data_hash: self.data_hash,
        }
    }

    pub fn to_node(&self) -> Node {
        let hashed_leaf = keccak::hashv(&[
            self.owner.as_ref(),
            self.delegate.as_ref(),
            self.nonce.to_le_bytes().as_ref(),
            self.data_hash.as_ref(),
        ])
        .to_bytes();
        Node::new(hashed_leaf)
    }
}