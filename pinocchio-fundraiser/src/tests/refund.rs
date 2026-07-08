#[cfg(test)]
pub mod refund {
    use core::fmt::Error;

    use crate::instructions::FundraiserInstruction;
    use crate::state::Contributor;
    use crate::tests::tests::{program_id, ReusableState};
    use solana_program_pack::Pack;
    use litesvm::LiteSVM;
    use litesvm_token::{CreateAssociatedTokenAccount, MintTo};
    use solana_clock::Clock;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    pub fn refund_function(svm: &mut LiteSVM, state: &ReusableState) -> Result<(), Error> {
        let ReusableState {
            maker,
            fundraiser,
            mint,
            vault,
            token_program,
            ..
        } = state;

        let amount_to_donate: u64 = 100_000_000; // 100 usdc - less than 10k target

        let contributor = Keypair::new();

        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let contributor_ata = CreateAssociatedTokenAccount::new(svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();

        // Mint some initial tokens to contributor
        let initial_contributor_balance = 1_000_000_000;
        MintTo::new(svm, &maker, &mint, &contributor_ata, initial_contributor_balance)
            .send()
            .unwrap();

        let contributor_pda = solana_pubkey::Pubkey::find_program_address(
            &[b"contributor".as_ref(), contributor.pubkey().as_ref()],
            &program_id(),
        );

        let donate_data = Contributor {
            amount: amount_to_donate.to_le_bytes(),
        };

        // 1. Contribute to campaign (under target)
        let donate_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new(*mint, false),
                AccountMeta::new(fundraiser.0, false),
                AccountMeta::new(contributor_pda.0, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(solana_sdk_ids::system_program::ID, false),
                AccountMeta::new(*token_program, false),
                AccountMeta::new(spl_associated_token_account::ID, false),
                AccountMeta::new(solana_sdk_ids::sysvar::rent::ID, false),
            ],
            data: [
                (FundraiserInstruction::Deposit as u8)
                    .to_le_bytes()
                    .to_vec(),
                donate_data.to_bytes().to_vec(),
            ]
            .concat(),
        };

        let message_donate = Message::new(&[donate_ix], Some(&contributor.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction_donate = Transaction::new(&[&contributor], message_donate, recent_blockhash);
        svm.send_transaction(transaction_donate).unwrap();

        // 2. Warp/advance time past duration
        // duration is 3 days
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp += 3 * 24 * 60 * 60 + 3600; // 3 days + 1 hour
        svm.set_sysvar::<Clock>(&clock);

        // 3. Perform Refund
        let refund_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(*mint, false),
                AccountMeta::new(fundraiser.0, false),
                AccountMeta::new(contributor_pda.0, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*token_program, false),
            ],
            data: vec![FundraiserInstruction::Refund as u8],
        };

        let message_refund = Message::new(&[refund_ix], Some(&contributor.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction_refund = Transaction::new(&[&contributor], message_refund, recent_blockhash);

        println!("\nSending Refund transaction...");
        let tx = svm.send_transaction(transaction_refund).unwrap();
        println!("Refund transaction successful. CUs Consumed: {}", tx.compute_units_consumed);

        // Verify that contributor state account was closed (deleted or data empty)
        let contributor_pda_acc = svm.get_account(&contributor_pda.0);
        assert!(
            contributor_pda_acc.is_none() || contributor_pda_acc.unwrap().data.is_empty(),
            "Contributor PDA account was not closed"
        );

        // Verify contributor ATA got their tokens back (original balance)
        let contributor_ata_acc = svm.get_account(&contributor_ata).unwrap();
        let contributor_ata_state = litesvm_token::spl_token::state::Account::unpack_from_slice(&contributor_ata_acc.data).unwrap();
        assert_eq!(
            contributor_ata_state.amount, initial_contributor_balance,
            "Contributor did not receive the refunded tokens"
        );

        Ok(())
    }
}
