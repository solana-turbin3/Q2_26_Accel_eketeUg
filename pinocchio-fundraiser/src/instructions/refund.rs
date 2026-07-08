use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_token::instructions::Transfer;

use crate::state::{Contributor, Fundraiser};

pub fn process_refund_instruction(accounts: &[AccountView]) -> ProgramResult {
    let [contributor, contributor_ata, mint_to_raise, fundraiser, contributor_state_account, vault, _token_program, _extra @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    assert!(contributor.is_signer(), "Contributor must sign");

    let fundraiser_data = fundraiser.try_borrow().unwrap();
    let fundraiser_state = bytemuck::from_bytes::<Fundraiser>(&*fundraiser_data);

    assert!(fundraiser.owned_by(&crate::ID), "Invalid Fundraiser");
    assert_eq!(
        mint_to_raise.address().as_ref(),
        &fundraiser_state.mint_to_raise,
        "Invalid Mint"
    );
    assert_eq!(
        vault.address().as_ref(),
        &fundraiser_state.vault,
        "Invalid Vault"
    );

    // Verify campaign is expired and goal was not met
    let current_time_unix = Clock::get()?.unix_timestamp;
    let time_started = u64::from_le_bytes(fundraiser_state.time_started);
    let duration_days = u8::from_le_bytes(fundraiser_state.duration);
    let target_days_in_unix: u64 = u64::from(duration_days) * 24 * 60 * 60;
    let expiration_time = time_started + target_days_in_unix;

    assert!(
        current_time_unix as u64 > expiration_time,
        "Campaign has not expired yet"
    );

    let current_amount = u64::from_le_bytes(fundraiser_state.current_amount);
    let amount_to_raise = u64::from_le_bytes(fundraiser_state.amount_to_raise);

    assert!(
        current_amount < amount_to_raise,
        "Goal was met, cannot refund"
    );

    // Verify contributor_state_account PDA address matches expected
    let seed = [b"contributor", contributor.address().as_ref()];
    let (created_contributor, _contributor_bump) = Address::find_program_address(&seed, &crate::ID);
    assert!(
        &created_contributor.eq(contributor_state_account.address()),
        "Invalid contributor state account"
    );

    // Read the contributed amount
    let contributor_data = contributor_state_account.try_borrow().unwrap();
    let contributor_state = bytemuck::from_bytes::<Contributor>(&*contributor_data);
    let refund_amount = u64::from_le_bytes(contributor_state.amount);

    assert!(refund_amount > 0, "No contribution to refund");

    // Prepare signer seeds for the fundraiser PDA
    let bump = fundraiser_state.bump;
    let maker_bytes = fundraiser_state.maker;
    let pda_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(&maker_bytes),
        Seed::from(&bump),
    ];

    // Drop the borrows before performing token transfer/closing accounts
    drop(contributor_data);
    drop(fundraiser_data);

    // Refund tokens to contributor from vault
    Transfer {
        amount: refund_amount,
        authority: fundraiser,
        from: vault,
        to: contributor_ata,
    }
    .invoke_signed(&[Signer::from(&pda_seeds)])?;

    // Close the contributor state account to return lamports
    let contributor_lamports = contributor.lamports();
    let contributor_state_lamports = contributor_state_account.lamports();
    contributor.set_lamports(contributor_lamports + contributor_state_lamports);
    contributor_state_account.set_lamports(0);
    contributor_state_account.close()?;

    Ok(())
}
