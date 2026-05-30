use anchor_lang::prelude::*;
use crate::state::Pet;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = Pet::SIZE,
        seeds = [b"pet"],
        bump
    )]
    pub pet: Account<'info, Pet>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>, name: String, task_queue: Pubkey) -> Result<()> {
    let pet = &mut ctx.accounts.pet;
    
    let name_bytes = name.as_bytes();
    let mut name_arr = [0u8; 32];
    let len = name_bytes.len().min(32);
    name_arr[..len].copy_from_slice(&name_bytes[..len]);
    
    pet.name = name_arr;
    pet.hunger = 0;
    pet.happiness = 100;
    pet.last_tick_timestamp = Clock::get()?.unix_timestamp;
    pet.is_alive = true;
    pet.task_queue = task_queue;
    pet.task_id = 0;
    pet.bump = ctx.bumps.pet;
    
    msg!("Pet {} initialized with task queue: {:?}", name, task_queue);
    Ok(())
}
