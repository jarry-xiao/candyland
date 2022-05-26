mod instructions;
mod logs;
mod storage;

pub use instructions::*;
pub use logs::*;
pub use storage::*;

use flatbuffers::{ForwardsUOffset, Vector};
use plerkle_serialization::transaction_info_generated::transaction_info;
use solana_sdk::pubkey::Pubkey;

pub fn un_jank_message(hex_str: &String) -> String {
    String::from_utf8(hex::decode(hex_str).unwrap()).unwrap()
}

pub fn pubkey_from_fb_table(
    keys: &Vector<ForwardsUOffset<transaction_info::Pubkey>>,
    index: usize,
) -> String {
    let pubkey = keys.get(index);
    Pubkey::new(pubkey.key().unwrap()).to_string()
}
