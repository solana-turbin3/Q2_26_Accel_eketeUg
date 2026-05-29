pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
mod tests;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("AkTTSsoAmjbmDQTVzFmoEWJ5o2j78xGBtSVuB68irJiJ");

#[program]
pub mod week1_challenge {
    use super::*;

    pub fn create_vault(ctx: Context<VaultOperation>) -> Result<()> {
        ctx.accounts.create_vault(&ctx.bumps)
    }

    pub fn mint_token(ctx: Context<TokenFactory>, amount: u64, decimals: u8) -> Result<()> {
        ctx.accounts.mint_to_admin(amount, decimals)
    }

    pub fn add_to_whitelist(
        ctx: Context<WhitelistOperations>,
        address: Pubkey,
        _mint: Pubkey,
    ) -> Result<()> {
        ctx.accounts.add_to_whitelist(address, &ctx.bumps)
    }

    pub fn remove_from_whitelist(
        ctx: Context<RemoveWhitelistOperations>,
        address: Pubkey,
        _mint: Pubkey,
    ) -> Result<()> {
        ctx.accounts.remove_from_whitelist(address)
    }

    // deposit
    pub fn deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositWithdraw<'info>>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.deposit(amount, &ctx.remaining_accounts)
    }
    // withdraw
    pub fn withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositWithdraw<'info>>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.withdraw(amount, &ctx.remaining_accounts)
    }
}

// create_vault ->
// deposit
// withdraw
// transfer_hook

// logic to check will be in transfer_hook
// mint of the vault will have token extension
// Don't forget to mint the token directly in the program
