use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    pub address: Pubkey,
    pub amount: u64,
    pub is_whitelisted: bool,
    pub entry_bump: u8,
}
