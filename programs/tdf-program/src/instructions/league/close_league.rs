use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::state::{League, LeagueStatus};

pub fn close_league(ctx: Context<CloseLeague>) -> Result<()> {
    let league = &mut ctx.accounts.league;

    // Check if the league is active
    require!(
        league.status == LeagueStatus::Active,
        crate::errors::ErrorCode::InvalidLeagueStatus
    );

    // If end time is not reached, only creator can close the league
    // But, if end time is reached, anyone can close the league
    let now = Clock::get()?.unix_timestamp;
    if now < league.end_ts {
        require!(
            ctx.accounts.user.key() == league.creator,
            crate::errors::ErrorCode::NotLeagueCreator
        );
    }

    // Verify the reward vault matches the league's reward vault
    require_keys_eq!(
        ctx.accounts.reward_vault.key(),
        league.reward_vault,
        crate::errors::ErrorCode::InvalidRewardVault
    );

    // Fix the total reward amount at the time of closing
    league.total_reward_amount = ctx.accounts.reward_vault.amount;
    league.status = LeagueStatus::Closed;

    msg!(
        "League {:?} closed with total reward amount: {}",
        league.key(),
        league.total_reward_amount
    );

    Ok(())
}

#[derive(Accounts)]
pub struct CloseLeague<'info> {
    #[account(mut)]
    pub league: Account<'info, League>,

    /// CHECK:
    pub reward_vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,
}
