use anchor_lang::prelude::*;
use anchor_spl::associated_token::{get_associated_token_address, AssociatedToken};
use anchor_spl::token::Token;

use crate::state::{
    Leaderboard, League, LeagueStatus, LEADERBOARD_SEED, LEADERBOARD_SPACE, LEAGUE_SEED,
    LEAGUE_SPACE,
};

pub fn create_league(
    ctx: Context<CreateLeague>,
    id: String,
    markets: Vec<Pubkey>,
    entry_amount: i64,
    virtual_on_deposit: i64,
    start_ts: i64,
    end_ts: i64,
    metadata_uri: String,
    max_participants: u32,
    max_leverage: u8,
    k: u16,
) -> Result<()> {
    // validate inputs
    require!(
        markets.len() <= 10,
        crate::errors::ErrorCode::InvalidMarketLength
    );
    require!(
        start_ts < end_ts,
        crate::errors::ErrorCode::InvalidTimeRange
    );
    require!(k <= 10, crate::errors::ErrorCode::InvalidKValue);

    let league = &mut ctx.accounts.league;
    let leaderboard = &mut ctx.accounts.leaderboard;
    let entry_token_mint_key = ctx.accounts.entry_token_mint.key();

    let reward_vault_ata = get_associated_token_address(&league.key(), &entry_token_mint_key);

    require_keys_eq!(
        ctx.accounts.reward_vault.key(),
        reward_vault_ata,
        crate::errors::ErrorCode::InvalidRewardVault
    );

    // Check if the ATA account exists and has data
    let ata_account_info = &ctx.accounts.reward_vault;
    if ata_account_info.data_is_empty() {
        // Create the associated token account if it doesn't exist
        anchor_spl::associated_token::create(CpiContext::new(
            ctx.accounts.associated_token_program.to_account_info(),
            anchor_spl::associated_token::Create {
                payer: ctx.accounts.creator.to_account_info(),
                associated_token: ctx.accounts.reward_vault.to_account_info(),
                authority: league.to_account_info(),
                mint: ctx.accounts.entry_token_mint.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
                token_program: ctx.accounts.token_program.to_account_info(),
            },
        ))?;

        msg!(
            "Created associated token account for league: {:?}",
            reward_vault_ata
        );
    } else {
        msg!(
            "Associated token account already exists: {:?}",
            reward_vault_ata
        );
    }

    league.id = id;
    league.creator = ctx.accounts.creator.key();
    league.status = LeagueStatus::Pending;
    league.markets = markets;
    league.leaderboard = leaderboard.key();
    league.entry_token_mint = entry_token_mint_key;
    league.entry_amount = entry_amount;
    league.reward_vault = ctx.accounts.reward_vault.key();
    league.total_reward_amount = 0;
    league.virtual_on_deposit = virtual_on_deposit;
    league.metadata_uri = metadata_uri;
    league.start_ts = start_ts;
    league.end_ts = end_ts;
    league.max_participants = max_participants;
    league.max_leverage = max_leverage;
    league.bump = ctx.bumps.league;

    leaderboard.league = league.key();
    leaderboard.k = k;
    leaderboard.topk_equity = vec![];
    leaderboard.topk_equity_scores = vec![];
    leaderboard.topk_volume = vec![];
    leaderboard.topk_volume_scores = vec![];
    leaderboard.last_updated = Clock::get()?.unix_timestamp;
    leaderboard.bump = ctx.bumps.leaderboard;

    Ok(())
}

#[derive(Accounts)]
#[instruction(id: String)]
pub struct CreateLeague<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = LEAGUE_SPACE,
        seeds = [LEAGUE_SEED, creator.key().as_ref(), id.as_ref()],
        bump
    )]
    pub league: Account<'info, League>,

    #[account(
        init,
        payer = creator,
        space = LEADERBOARD_SPACE,
        seeds = [LEADERBOARD_SEED, league.key().as_ref()],
        bump
    )]
    pub leaderboard: Account<'info, Leaderboard>,

    /// CHECK:
    pub entry_token_mint: AccountInfo<'info>,

    /// CHECK:
    #[account(mut)]
    pub reward_vault: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}
