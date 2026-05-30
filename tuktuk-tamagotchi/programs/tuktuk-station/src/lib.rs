pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use state::*;
use instructions::*;

declare_id!("5sPxfWKmtCa4wX33ouEbaFV6Kmb5MLaJaSFEnDHRaVXs");

#[program]
pub mod tuktuk_station {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, name: String, task_queue: Pubkey) -> Result<()> {
        instructions::initialize::initialize(ctx, name, task_queue)
    }

    pub fn schedule_next_crank(ctx: Context<ScheduleNextCrank>, task_id: u16) -> Result<()> {
        instructions::schedule_next_crank::schedule_next_crank(ctx, task_id)
    }

    pub fn increment_counter(ctx: Context<IncrementCounter>, task_id: u16) -> Result<()> {
        instructions::increment_counter::increment_counter(ctx, task_id)
    }

    pub fn feed_pet(ctx: Context<FeedPet>) -> Result<()> {
        instructions::feed_pet::feed_pet(ctx)
    }

    pub fn play_with_pet(ctx: Context<PlayWithPet>) -> Result<()> {
        instructions::play_with_pet::play_with_pet(ctx)
    }
}
