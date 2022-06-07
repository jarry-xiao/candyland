use crate::error::BubblegumError;
use anchor_lang::prelude::*;

pub const GUMMYROLL_TREE_AUTHORITY_SIZE: usize = 304 + 8;
pub const APPEND_ALLOWLIST_SIZE: usize = 5;
#[account]
pub struct GummyrollTreeAuthority {
    /// Pubkey of merkle roll that this manages
    pub tree_id: Pubkey,
    /// How many NFTs have been minted
    pub count: u64,
    /// Always able to transfer owner, delegate, modify append_allowlist
    pub owner: Pubkey,
    /// Always able to transfer delegate, modify append_allowlist
    pub delegate: Pubkey,
    /// Able to append up to corresponding # of uses via bubblegum
    pub append_allowlist: [AppendAllowlistEntry; APPEND_ALLOWLIST_SIZE],
}

impl GummyrollTreeAuthority {
    pub fn increment_allowlist(&mut self, allowlist_pubkey: &Pubkey, amount: u64) -> Result<()> {
        match self
            .append_allowlist
            .iter()
            .position(|&entry| entry.pubkey == *allowlist_pubkey)
        {
            Some(idx) => match self.append_allowlist[idx].num_appends.checked_add(amount) {
                Some(new_num_appends) => {
                    self.append_allowlist[idx].num_appends = new_num_appends;
                    Ok(())
                }
                None => {
                    err!(BubblegumError::AppendAllowlistIncrementOverflow)
                }
            },
            None => {
                err!(BubblegumError::AppendAuthorityNotFound)
            }
        }
    }

    pub fn decrement_allowlist(&mut self, allowlist_pubkey: &Pubkey, amount: u64) -> Result<()> {
        match self
            .append_allowlist
            .iter()
            .position(|&entry| entry.pubkey == *allowlist_pubkey)
        {
            Some(idx) => match self.append_allowlist[idx].num_appends.checked_sub(amount) {
                Some(new_num_appends) => {
                    self.append_allowlist[idx].num_appends = new_num_appends;
                    if new_num_appends == 0 {
                        return self.remove_append_authority(allowlist_pubkey);
                    }
                    Ok(())
                }
                None => {
                    err!(BubblegumError::AppendAllowlistIncrementUnderflow)
                }
            },
            None => {
                err!(BubblegumError::AppendAuthorityNotFound)
            }
        }
    }

    pub fn remove_append_authority(&mut self, allowlist_pubkey: &Pubkey) -> Result<()> {
        let mut allowlist = self.append_allowlist.to_vec();
        let mut entries = allowlist.iter();
        match entries.position(|&allowlist_entry| allowlist_entry.pubkey == *allowlist_pubkey) {
            Some(idx_to_remove) => {
                allowlist.swap_remove(idx_to_remove);
                allowlist.push(AppendAllowlistEntry::default());
                self.append_allowlist[..allowlist.len()].copy_from_slice(&allowlist);
                return Ok(());
            }
            None => {
                return err!(BubblegumError::AppendAuthorityNotFound);
            }
        }
    }
}

#[repr(C)]
#[derive(AnchorDeserialize, AnchorSerialize, Default, Debug, Copy, Clone)]
pub struct AppendAllowlistEntry {
    pub pubkey: Pubkey,
    pub num_appends: u64,
}
