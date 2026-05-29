use anchor_lang::prelude::*;
use crate::state::UserAccount;

#[derive(Accounts)]
pub struct UpdateVrf<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump
    )]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> UpdateVrf<'info> {
    pub fn update_vrf(&mut self) -> Result<()> {
        let clock = Clock::get()?;
        let random_number = clock.unix_timestamp as u64; // Pseudo-random for demonstration
        self.user_account.data = random_number;
        Ok(())
    }
}
