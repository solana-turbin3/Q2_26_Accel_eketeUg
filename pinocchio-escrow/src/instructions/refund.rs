use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_pubkey::derive_address;

use crate::state::Escrow;

pub fn process_refund_instruction(accounts: &mut [AccountView], _data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_a,
        escrow_account,
        maker_ata,
        escrow_ata,
        _system_program,
        _token_program,
        _extra @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (maker_address, mint_a_address, amount_to_give, bump) = {
        let escrow_state = Escrow::from_account_info(escrow_account)?;
        (
            *escrow_state.maker(),
            *escrow_state.mint_a(),
            escrow_state.amount_to_give(),
            escrow_state.bump,
        )
    };

    if maker_address != *maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }
    if mint_a_address != *mint_a.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    // Verify escrow_ata's owner and mint
    {
        let escrow_ata_state = pinocchio_token::state::Account::from_account_view(escrow_ata)?;
        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if escrow_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    // Transfer Token A from escrow_ata vault back to maker_ata
    pinocchio_token::instructions::Transfer::new(
        escrow_ata,
        maker_ata,
        escrow_account,
        amount_to_give,
    )
    .invoke_signed(&[signer.clone()])?;

    // Close escrow_ata and return rent lamports to maker
    pinocchio_token::instructions::CloseAccount::new(
        escrow_ata,
        maker,
        escrow_account,
    )
    .invoke_signed(&[signer])?;

    // Reclaim lamports from the escrow state account to the maker, then close it
    let escrow_lamports = escrow_account.lamports();
    escrow_account.set_lamports(0);
    maker.set_lamports(maker.lamports() + escrow_lamports);

    escrow_account.close()?;

    Ok(())
}
