use anchor_lang::prelude::*;

#[account]
pub struct Pet {
    pub name: [u8; 32],
    pub hunger: u8,
    pub happiness: u8,
    pub last_tick_timestamp: i64,
    pub is_alive: bool,
    pub task_queue: Pubkey,
    pub task_id: u16,
    pub bump: u8,
}

impl Pet {
    pub const SIZE: usize = 8 + 32 + 1 + 1 + 8 + 1 + 32 + 2 + 1;
}
