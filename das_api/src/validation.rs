use crate::DasApiError;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const PUBKEY_CHARS: usize = 44;

pub fn validate_pubkey(str_pubkey: String) -> Result<Pubkey, DasApiError> {
    if str_pubkey.len() == PUBKEY_CHARS {
        return Pubkey::from_str(&*str_pubkey)
            .map_err(|e| DasApiError::PubkeyValidationError(str_pubkey));
    }
    Err(DasApiError::ValidationError(format!(
        "{} is not a valid length of {}",
        str_pubkey, PUBKEY_CHARS
    )))
}
