#[cfg(test)]
pub mod claim {
    use core::fmt::Error;

    use crate::instructions::FundraiserInstruction;
    use crate::state::Contributor;
    use crate::tests::tests::{program_id, ReusableState};
    use solana_program_pack::Pack;
    use litesvm::LiteSVM;
    use litesvm_token::{CreateAssociatedTokenAccount, MintTo};
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_signer::Signer;
    use solana_transaction::Transaction;

    pub fn claim_function(svm: &mut LiteSVM, state: &ReusableState) -> Result<(), Error> {
        let ReusableState {
            maker,
            fundraiser,
            mint,
            vault,
            token_program,
            maker_ata,
            ..
        } = state;

        let amount_to_donate: u64 = 10_000_000_000; // 10k usdc - meets the goal exactly

        let contributor = Keypair::new();

        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let contributor_ata = CreateAssociatedTokenAccount::new(svm, &contributor, &mint)
            .owner(&contributor.pubkey())
            .send()
            .unwrap();

        // Mint exact amount to donate
        MintTo::new(svm, &maker, &mint, &contributor_ata, amount_to_donate)
            .send()
            .unwrap();

        let contributor_pda = solana_pubkey::Pubkey::find_program_address(
            &[b"contributor".as_ref(), contributor.pubkey().as_ref()],
            &program_id(),
        );

        let donate_data = Contributor {
            amount: amount_to_donate.to_le_bytes(),
        };

        // 1. Contribute enough to meet the goal
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

        // 2. Perform Claim
        let claim_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(*maker_ata, false),
                AccountMeta::new(*mint, false),
                AccountMeta::new(fundraiser.0, false),
                AccountMeta::new(*vault, false),
                AccountMeta::new(*token_program, false),
            ],
            data: vec![FundraiserInstruction::Claim as u8],
        };

        let message_claim = Message::new(&[claim_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction_claim = Transaction::new(&[maker], message_claim, recent_blockhash);

        println!("\nSending Claim transaction...");
        let tx = svm.send_transaction(transaction_claim).unwrap();
        println!("Claim transaction successful. CUs Consumed: {}", tx.compute_units_consumed);

        // Verify that the fundraiser state account was closed (it is either deleted or has 0 data)
        let fundraiser_acc = svm.get_account(&fundraiser.0);
        assert!(
            fundraiser_acc.is_none() || fundraiser_acc.unwrap().data.is_empty(),
            "Fundraiser account was not closed"
        );

        // Verify that the vault account was closed (it is either deleted or has 0 data)
        let vault_acc = svm.get_account(vault);
        assert!(
            vault_acc.is_none() || vault_acc.unwrap().data.is_empty(),
            "Vault account was not closed"
        );

        // Verify maker_ata got the tokens
        let maker_ata_acc = svm.get_account(maker_ata).unwrap();
        let maker_ata_state = litesvm_token::spl_token::state::Account::unpack_from_slice(&maker_ata_acc.data).unwrap();
        assert_eq!(
            maker_ata_state.amount, amount_to_donate,
            "Maker did not receive the claimed tokens"
        );

        Ok(())
    }
}
