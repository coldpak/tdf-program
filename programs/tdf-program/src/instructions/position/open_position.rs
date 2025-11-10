/// CAUTION: This instruction is applied only to ER. 
/// Base Layer's price feed is not updated in realtime.
use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;

use crate::state::{
    Direction, League, LeagueStatus, Market, Participant, Position, PARTICIPANT_SEED, POSITION_SEED, POSITION_SPACE
};
use crate::utils::get_price_from_pyth;

/// Initialize Position just for delegation
pub fn init_unopened_position(
    ctx: Context<InitUnopenedPosition>, 
    league: Pubkey,
    current_position_seq: u64,
    market: Pubkey,
    market_decimals: u8,
) -> Result<()> {
    // TODO: Check if participant is in the league, user is the participant etc...

    let position = &mut ctx.accounts.position;
    position.league = league;
    position.user = ctx.accounts.user.key();
    position.market = market;
    position.market_decimals = market_decimals;
    position.price_feed = ctx.accounts.price_feed.key();
    position.seq_num = current_position_seq;

    position.bump = ctx.bumps.position;

    Ok(())
}

pub fn delegate_unopened_position(ctx: Context<DelegateUnopenedPosition>, participant: Pubkey, position_seq: u64) -> Result<()> {
  // TODO: Check if participant is in the league, user is the participant etc...
  ctx.accounts.delegate_position(
        &ctx.accounts.user,
        &[
            POSITION_SEED, 
            ctx.accounts.league.key().as_ref(),
            ctx.accounts.user.key().as_ref(),
            position_seq.to_le_bytes().as_ref()
        ],
        DelegateConfig {
          ..Default::default()
        },
    )?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(league: Pubkey, current_position_seq: u64)]
pub struct InitUnopenedPosition<'info> {
    #[account(
        init,
        payer = user,
        space = POSITION_SPACE,
        seeds = [
            POSITION_SEED, 
            league.as_ref(), 
            user.key().as_ref(), 
            current_position_seq.to_le_bytes().as_ref()
        ],
        bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: Price feed account (Pyth PriceUpdateV2)
    pub price_feed: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateUnopenedPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub league: Account<'info, League>,

    /// CHECK: Position account
    #[account(mut, del)]
    pub position: AccountInfo<'info>,
}


#[commit]
#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
}