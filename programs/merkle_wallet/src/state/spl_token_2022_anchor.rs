
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::program_pack::Pack;
use anchor_lang::solana_program::pubkey::Pubkey;
use std::ops::Deref;


#[derive(Clone)]
pub struct TokenAccount(spl_token_2022::state::Account);

impl TokenAccount {
    pub const LEN: usize = spl_token_2022::state::Account::LEN;
}

impl anchor_lang::AccountDeserialize for TokenAccount {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        spl_token_2022::state::Account::unpack(buf).map(TokenAccount)
    }
}

impl anchor_lang::AccountSerialize for TokenAccount {}

impl anchor_lang::Owner for TokenAccount {
    fn owner() -> Pubkey {
        spl_token_2022::id()
    }
}

impl Deref for TokenAccount {
    type Target = spl_token_2022::state::Account;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Mint(spl_token_2022::state::Mint);

impl Mint {
    pub const LEN: usize = spl_token_2022::state::Mint::LEN;
}

impl anchor_lang::AccountDeserialize for Mint {
    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        spl_token_2022::state::Mint::unpack(buf).map(Mint)
    }
}

impl anchor_lang::AccountSerialize for Mint {}

impl anchor_lang::Owner for Mint {
    fn owner() -> Pubkey {
        spl_token_2022::id()
    }
}

impl Deref for Mint {
    type Target = spl_token_2022::state::Mint;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct Token2022;

impl anchor_lang::Id for Token2022 {
    fn id() -> Pubkey {
        spl_token_2022::id()
    }
}