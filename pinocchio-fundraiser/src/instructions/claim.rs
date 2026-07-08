use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};
use pinocchio_token::{
    instructions::{CloseAccount, Transfer},
    state::TokenAccount,
};

use crate::state::Fundraiser;

pub fn process_claim_instruction(accounts: &[AccountView]) -> ProgramResult {
    let [maker, maker_ata, mint_to_raise, fundraiser, vault, _token_program, _extra @ ..] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    assert!(maker.is_signer(), "Maker must sign");

    let fundraiser_data = fundraiser.try_borrow().unwrap();
    let fundraiser_state = bytemuck::from_bytes::<Fundraiser>(&*fundraiser_data);

    assert!(fundraiser.owned_by(&crate::ID), "Invalid Fundraiser");
    assert_eq!(
        maker.address().as_ref(),
        &fundraiser_state.maker,
        "Invalid Maker"
    );
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

    let current_amount = u64::from_le_bytes(fundraiser_state.current_amount);
    let amount_to_raise = u64::from_le_bytes(fundraiser_state.amount_to_raise);

    assert!(
        current_amount >= amount_to_raise,
        "Goal was not met"
    );

    // Prepare signer seeds for the fundraiser PDA
    let bump = fundraiser_state.bump;
    let maker_bytes = fundraiser_state.maker;
    let pda_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(&maker_bytes),
        Seed::from(&bump),
    ];

    // Read the balance of the vault to transfer it out
    let transfer_amount = {
        let vault_as_state = TokenAccount::from_account_view(vault).unwrap();
        vault_as_state.amount()
    };

    // Drop fundraiser_data reference so we can mutate/close fundraiser
    drop(fundraiser_data);

    if transfer_amount > 0 {
        Transfer {
            amount: transfer_amount,
            authority: fundraiser,
            from: vault,
            to: maker_ata,
        }
        .invoke_signed(&[Signer::from(&pda_seeds)])?;
    }

    // Close the vault account
    CloseAccount {
        account: vault,
        destination: maker,
        authority: fundraiser,
    }
    .invoke_signed(&[Signer::from(&pda_seeds)])?;

    // Close the fundraiser state account
    let maker_lamports = maker.lamports();
    let fundraiser_lamports = fundraiser.lamports();
    maker.set_lamports(maker_lamports + fundraiser_lamports);
    fundraiser.set_lamports(0);
    fundraiser.close()?;

    Ok(())
}
