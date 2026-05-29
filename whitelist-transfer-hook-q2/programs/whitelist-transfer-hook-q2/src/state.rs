use anchor_lang::prelude::*;

#[account]
pub struct WhitelistEntry {
    pub bump: u8,
}
