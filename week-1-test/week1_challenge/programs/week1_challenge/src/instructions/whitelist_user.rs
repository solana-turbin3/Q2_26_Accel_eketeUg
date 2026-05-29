use anchor_lang::prelude::*;

use crate::{state::whitelist::WhitelistEntry, Vault};

use crate::error::VaultError;
use crate::instructions::VAULT_SEED;

#[derive(Accounts)]
#[instruction(address: Pubkey, mint: Pubkey)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
        constraint = admin.key() == vault.owner.key() @VaultError::NotAdmin
    )]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        seeds = [b"whitelist", mint.key().as_ref(), address.as_ref()],
        space = 8 + WhitelistEntry::INIT_SPACE,
        bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    #[account(
        mut @VaultError::VaultNotCreatedByAdmin,
        seeds = [mint.key().as_ref(), VAULT_SEED],
        bump = vault.vault_bump,        
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(address: Pubkey, mint: Pubkey)]
pub struct RemoveWhitelistOperations<'info> {
    #[account(
        mut,
        constraint = admin.key() == vault.owner.key() @VaultError::NotAdmin
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        close = admin,
        seeds = [b"whitelist", mint.key().as_ref(), address.as_ref()],
        bump = whitelist_entry.entry_bump,
    )]
    pub whitelist_entry: Account<'info, WhitelistEntry>,

    #[account(
        mut @VaultError::VaultNotCreatedByAdmin,
        seeds = [mint.key().as_ref(), VAULT_SEED],
        bump = vault.vault_bump,        
    )]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}


impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, address: Pubkey, bumps: &WhitelistOperationsBumps) -> Result<()> {
        self.whitelist_entry.set_inner(WhitelistEntry {
            address,
            amount: 0,
            is_whitelisted: true,
            entry_bump: bumps.whitelist_entry,
        });
        Ok(())
    }
}

impl<'info> RemoveWhitelistOperations<'info> {
    pub fn remove_from_whitelist(&mut self, _address: Pubkey) -> Result<()> {
        // Account will be closed automatically by Anchor
        Ok(())
    }
}
