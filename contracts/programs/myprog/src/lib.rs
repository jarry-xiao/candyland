use anchor_lang::prelude::*;

declare_id!("Nhx3SJWLJCmuGjHfzjUd4J4Fj1HUncfumsRn12f2GRF");

#[program]
pub mod myprog {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
