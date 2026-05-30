use anchor_lang::prelude::*;
use crate::state::Pet;
use crate::error::ErrorCode;

#[derive(Accounts)]
pub struct FeedPet<'info> {
    #[account(
        mut,
        seeds = [b"pet"],
        bump = pet.bump,
    )]
    pub pet: Account<'info, Pet>,
    pub owner: Signer<'info>,
}

pub fn feed_pet(ctx: Context<FeedPet>) -> Result<()> {
    let pet = &mut ctx.accounts.pet;
    
    require!(pet.is_alive, ErrorCode::PetIsDead);
    
    if pet.hunger > 25 {
        pet.hunger -= 25;
    } else {
        pet.hunger = 0;
    }
    
    pet.last_tick_timestamp = Clock::get()?.unix_timestamp;
    
    // Clean name bytes for message
    let name_str = String::from_utf8_lossy(&pet.name);
    let name_trimmed = name_str.trim_end_matches('\0');
    msg!("You fed {}! Current hunger: {}", name_trimmed, pet.hunger);
    
    Ok(())
}
