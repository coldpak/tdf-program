use anchor_lang::prelude::*;

use crate::state::Position;

pub fn close_position(ctx: Context<ClosePosition>) -> Result<()> {
    Ok(())
}

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub position: Account<'info, Position>,
}
