use anchor_lang::solana_program::{msg, program_error::ProgramError};
use bytemuck::PodCastError;
use std::any::type_name;
use std::mem::size_of;

pub fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> ProgramError {
    move |_: PodCastError| -> ProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        ProgramError::InvalidAccountData
    }
}
