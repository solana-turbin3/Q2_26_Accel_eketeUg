pub mod contribute;
pub mod initialize;
pub mod claim;
pub mod refund;

pub use contribute::*;
pub use initialize::*;
pub use claim::*;
pub use refund::*;
use pinocchio::error::ProgramError;

pub enum FundraiserInstruction {
    Initialize = 0,
    Deposit = 1,
    Claim = 2,
    Refund = 3,
}

impl TryFrom<&u8> for FundraiserInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundraiserInstruction::Initialize),
            1 => Ok(FundraiserInstruction::Deposit),
            2 => Ok(FundraiserInstruction::Claim),
            3 => Ok(FundraiserInstruction::Refund),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
