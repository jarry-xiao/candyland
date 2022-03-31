use std::error::Error;
use anchor_client::anchor_lang;
use base64;
use crate::error::{ApiError};
use anchor_lang::{Event, AnchorDeserialize};

pub fn handle_event<T: anchor_lang::Event + anchor_lang::AnchorDeserialize>(data: String) -> Result<T, ApiError>{
    let borsh_bytes = match base64::decode(&data) {
        Ok(borsh_bytes) => borsh_bytes,
        _ => {
            return Err(ApiError::ChangeLogEventMalformed);
        }
    };

    let mut slice: &[u8] = &borsh_bytes[..];
    let disc: [u8; 8] = {
        let mut disc = [0; 8];
        disc.copy_from_slice(&borsh_bytes[..8]);
        slice = &slice[8..];
        disc
    };
    if disc != T::discriminator() {
        return Err(ApiError::ChangeLogEventMalformed);
    }

    let e: T = anchor_lang::AnchorDeserialize::deserialize(&mut slice)
        .map_err(|_| ApiError::ChangeLogEventMalformed)?;
       Ok(e)
}