use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token};

use crate::state::{League, LeagueStatus, Participant, PARTICIPANT_SEED, PARTICIPANT_SPACE};

pub fn join_league(ctx: Context<JoinLeague>) -> Result<()> {
    let league = &mut ctx.accounts.league;
    require!(
        league.status == LeagueStatus::Active,
        crate::errors::ErrorCode::InvalidLeagueStatus
    );

    // try transfer entry token to the league
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::Transfer {
                from: ctx.accounts.user_entry_token_account.to_account_info(),
                to: ctx.accounts.reward_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        league.entry_amount as u64,
    )?;

    let participant = &mut ctx.accounts.participant;

    participant.league = league.key();
    participant.user = ctx.accounts.user.key();
    participant.claimed = false;
    participant.virtual_balance = league.virtual_on_deposit;
    participant.positions = vec![];
    participant.topk_equity_index = 0xFFFF;
    participant.topk_volume_index = 0xFFFF;
    participant.bump = ctx.bumps.participant;

    Ok(())
}

#[derive(Accounts)]
pub struct JoinLeague<'info> {
    #[account(mut)]
    pub league: Account<'info, League>,

    #[account(
      init,
      payer = user,
      space = PARTICIPANT_SPACE,
      seeds = [
        PARTICIPANT_SEED, 
        league.key().as_ref(),
        user.key().as_ref(),
      ],
      bump
    )]
    pub participant: Account<'info, Participant>,

    /// CHECK:
    #[account(mut)]
    pub reward_vault: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub user_entry_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
