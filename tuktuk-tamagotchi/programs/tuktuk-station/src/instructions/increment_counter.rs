use anchor_lang::prelude::*;
use crate::state::Pet;
use crate::error::ErrorCode;
use tuktuk_program::tuktuk::cpi::accounts::QueueTaskV0;
use tuktuk_program::types::{CompiledTransactionV0, CompiledInstructionV0, TransactionSourceV0, TriggerV0, QueueTaskArgsV0};

#[derive(Accounts)]
#[instruction(task_id: u16)]
pub struct IncrementCounter<'info> {
    #[account(
        mut,
        seeds = [b"pet"],
        bump = pet.bump,
        has_one = task_queue,
    )]
    pub pet: Account<'info, Pet>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Derived queue authority PDA of our program
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: AccountInfo<'info>,

    /// CHECK: The TukTuk task queue account
    #[account(mut)]
    pub task_queue: AccountInfo<'info>,

    /// CHECK: The TukTuk task queue authority PDA
    pub task_queue_authority: AccountInfo<'info>,

    /// CHECK: The next TukTuk task account to be created. Derived from seeds [b"task", task_queue.key(), task_id.to_le_bytes()] under the tuktuk program.
    #[account(mut)]
    pub task: AccountInfo<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: The TukTuk program
    pub tuktuk_program: AccountInfo<'info>,
}

pub fn increment_counter(ctx: Context<IncrementCounter>, task_id: u16) -> Result<()> {
    let pet = &mut ctx.accounts.pet;
    
    // 1. Decay pet stats
    require!(pet.is_alive, ErrorCode::PetIsDead);

    if pet.hunger <= 90 {
        pet.hunger += 10;
    } else {
        pet.hunger = 100;
        pet.is_alive = false;
    }

    if pet.happiness >= 10 {
        pet.happiness -= 10;
    } else {
        pet.happiness = 0;
    }

    let now = Clock::get()?.unix_timestamp;
    pet.last_tick_timestamp = now;

    let name_str = String::from_utf8_lossy(&pet.name);
    let name_trimmed = name_str.trim_end_matches('\0');
    if pet.is_alive {
        msg!("{} decayed! Current Hunger: {}, Happiness: {}", name_trimmed, pet.hunger, pet.happiness);
    } else {
        msg!("Oh no! {} has starved to death!", name_trimmed);
    }

    // 2. Schedule the next task recursively
    let task_queue = &ctx.accounts.task_queue;
    let task_queue_authority = &ctx.accounts.task_queue_authority;
    let queue_authority = &ctx.accounts.queue_authority;
    let system_program = &ctx.accounts.system_program;
    let tuktuk_program = &ctx.accounts.tuktuk_program;

    let next_task_id = task_id + 1;
    let next_task_pda = Pubkey::find_program_address(
        &[
            b"task",
            task_queue.key().as_ref(),
            &next_task_id.to_le_bytes(),
        ],
        &tuktuk_program.key()
    ).0;

    // Trigger in 60 seconds
    let trigger = TriggerV0::Timestamp(now + 60);

    let args = QueueTaskArgsV0 {
        id: task_id,
        trigger,
        transaction: TransactionSourceV0::CompiledV0(CompiledTransactionV0 {
            num_rw_signers: 0,
            num_ro_signers: 0,
            num_rw: 4, // pet, task_queue, next_task_pda, queue_authority
            accounts: vec![
                pet.key(),
                task_queue.key(),
                next_task_pda,
                queue_authority.key(),
                task_queue_authority.key(),
                system_program.key(),
                tuktuk_program.key(),
                crate::id(),
            ],
            instructions: vec![CompiledInstructionV0 {
                program_id_index: 7,
                accounts: vec![0, 1, 4, 2, 3, 5, 6], // pet, task_queue, task_queue_authority, task, queue_authority, system_program, tuktuk_program
                data: {
                    let mut data = anchor_lang::solana_program::hash::hash("global:increment_counter".as_bytes()).to_bytes()[..8].to_vec();
                    data.extend_from_slice(&next_task_id.to_le_bytes());
                    data
                },
            }],
            signer_seeds: vec![],
        }),
        crank_reward: None,
        free_tasks: 0,
        description: "TukTuk Tamagotchi Decay Tick".to_string(),
    };

    let cpi_program = tuktuk_program.to_account_info();
    let cpi_accounts = QueueTaskV0 {
        payer: ctx.accounts.payer.to_account_info(),
        queue_authority: queue_authority.to_account_info(),
        task_queue_authority: task_queue_authority.to_account_info(),
        task_queue: task_queue.to_account_info(),
        task: ctx.accounts.task.to_account_info(),
        system_program: system_program.to_account_info(),
    };

    let bump = ctx.bumps.queue_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"queue_authority",
        &[bump],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
    tuktuk_program::tuktuk::cpi::queue_task_v0(cpi_ctx, args)?;

    pet.task_id = task_id;
    msg!("Decay tick task successfully scheduled! Current task_id: {}", task_id);

    Ok(())
}
