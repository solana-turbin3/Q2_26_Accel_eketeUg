use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer},
    error::ProgramError,
};
use pinocchio_pubkey::derive_address;

use crate::state::Escrow;

pub fn process_take_instruction(accounts: &mut [AccountView], _data: &[u8]) -> ProgramResult {
    let [
        taker,
        maker,
        mint_a,
        mint_b,
        escrow_account,
        taker_ata_b,
        taker_ata_a,
        maker_ata_b,
        escrow_ata,
        _system_program,
        _token_program,
        _extra @ ..,
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !taker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (maker_address, mint_a_address, mint_b_address, amount_to_receive, amount_to_give, bump) = {
        let escrow_state = Escrow::from_account_info(escrow_account)?;
        (
            *escrow_state.maker(),
            *escrow_state.mint_a(),
            *escrow_state.mint_b(),
            escrow_state.amount_to_receive(),
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
    if mint_b_address != *mint_b.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    // Verify token accounts
    {
        let escrow_ata_state = pinocchio_token::state::Account::from_account_view(escrow_ata)?;
        if escrow_ata_state.owner() != escrow_account.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if escrow_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    {
        let taker_ata_b_state = pinocchio_token::state::Account::from_account_view(taker_ata_b)?;
        if taker_ata_b_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    {
        let taker_ata_a_state = pinocchio_token::state::Account::from_account_view(taker_ata_a)?;
        if taker_ata_a_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_a_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    {
        let maker_ata_b_state = pinocchio_token::state::Account::from_account_view(maker_ata_b)?;
        if maker_ata_b_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // 1. Transfer Token B from taker_ata_b to maker_ata_b
    pinocchio_token::instructions::Transfer::new(
        taker_ata_b,
        maker_ata_b,
        taker,
        amount_to_receive,
    )
    .invoke()?;

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    // 2. Transfer Token A from escrow_ata to taker_ata_a
    pinocchio_token::instructions::Transfer::new(
        escrow_ata,
        taker_ata_a,
        escrow_account,
        amount_to_give,
    )
    .invoke_signed(&[signer.clone()])?;

    // 3. Close escrow_ata and return rent lamports to maker
    pinocchio_token::instructions::CloseAccount::new(
        escrow_ata,
        maker,
        escrow_account,
    )
    .invoke_signed(&[signer])?;

    // 4. Close escrow_account and return rent lamports to maker
    let escrow_lamports = escrow_account.lamports();
    escrow_account.set_lamports(0);
    maker.set_lamports(maker.lamports() + escrow_lamports);

    escrow_account.close()?;

    Ok(())
}
