use anchor_lang::prelude::*;

use crate::state::{League, LeagueStatus};

pub fn start_league(ctx: Context<StartLeague>) -> Result<()> {
  let league = &mut ctx.accounts.league;

  // Check if the league is pending
  require!(
      league.status == LeagueStatus::Pending,
      crate::errors::ErrorCode::InvalidLeagueStatus
  );

  // If start time is not reached, only creator can start the league
  // But if start time is reached, anyone can start the league
  let now = Clock::get()?.unix_timestamp;
  if now < league.start_ts {
      require!(
          ctx.accounts.user.key() == league.creator,
          crate::errors::ErrorCode::NotLeagueCreator
      );
  }

  league.status = LeagueStatus::Active;
  msg!("League {:?} started!", league.key());

  Ok(())
}

// Start league with delegation
#[derive(Accounts)]
pub struct StartLeague<'info> {
    #[account(mut)]
    pub league: Account<'info, League>,

    #[account(mut)]
    pub user: Signer<'info>,
}
