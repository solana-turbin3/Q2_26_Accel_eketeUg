use anchor_lang::prelude::*;
use crate::state::Pet;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct PlayWithPet<'info> {
    #[account(
        mut,
        seeds = [b"pet"],
        bump = pet.bump,
    )]
    pub pet: Account<'info, Pet>,
    pub owner: Signer<'info>,
}

pub fn play_with_pet(ctx: Context<PlayWithPet>) -> Result<()> {
    let pet = &mut ctx.accounts.pet;
    
    require!(pet.is_alive, ErrorCode::PetIsDead);
    
    if pet.happiness <= 75 {
        pet.happiness += 25;
    } else {
        pet.happiness = 100;
    }
    
    pet.last_tick_timestamp = Clock::get()?.unix_timestamp;
    
    let name_str = String::from_utf8_lossy(&pet.name);
    let name_trimmed = name_str.trim_end_matches('\0');
    msg!("You played with {}! Current happiness: {}", name_trimmed, pet.happiness);
    
    Ok(())
}
