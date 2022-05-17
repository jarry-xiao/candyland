use anchor_lang::prelude::*;
pub mod state;
pub mod utils;

declare_id!("BRKyVDRGT7SPBtMhjHN4PVSPVYoc3Wa3QTyuRVM4iZkt");

#[derive(Accounts)]
pub struct InitGumballMachine {}

#[derive(Accounts)]
pub struct UpdateConfigLine {}

#[derive(Accounts)]
pub struct UpdateConfigMetadata {}

#[derive(Accounts)]
pub struct Dispense {}

#[derive(Accounts)]
pub struct Destroy {}

#[program]
pub mod gumball_machine {
    use super::*;

    pub fn initialize_gumball_machine(_ctx: Context<InitGumballMachine>) -> Result<()> {
        Ok(())
    }

    pub fn update_config_line(_ctx: Context<UpdateConfigLine>) -> Result<()> {
        Ok(())
    }

    pub fn update_config_metadata(_ctx: Context<UpdateConfigMetadata>) -> Result<()> {
        Ok(())
    }

    pub fn dispense(_ctx: Context<Dispense>) -> Result<()> {
        Ok(())
    }

    pub fn destroy(_ctx: Context<Destroy>) -> Result<()> {
        Ok(())
    }
}
