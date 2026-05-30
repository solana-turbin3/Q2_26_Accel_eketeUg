use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Custom error message")]
    CustomError,
    #[msg("Alas! The pet is dead. You cannot feed or play with it anymore.")]
    PetIsDead,
}
