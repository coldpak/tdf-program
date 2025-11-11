/// CAUTION: This instruction is applied only to ER. 
/// Base Layer's price feed is not updated in realtime.
use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{delegate};
use ephemeral_rollups_sdk::cpi::DelegateConfig;

use magicblock_permission_client::instructions::{
    CreateGroupCpiBuilder, CreatePermissionCpiBuilder
};

use crate::state::{
    Direction, League, LeagueStatus, Market, Participant, Position, PARTICIPANT_SEED, POSITION_SEED, POSITION_SPACE
};
use crate::utils::{get_price_and_exponent_from_pyth, calculate_notional};
use crate::constants::QUOTE_DECIMALS;

/// Initialize Position just for delegation
pub fn init_unopened_position(
    ctx: Context<InitUnopenedPosition>, 
    league: Pubkey,
    current_position_seq: u64,
    market: Pubkey,
    market_decimals: u8,
) -> Result<()> {
    // TODO: Check if participant is in the league, user is the participant, market decimal etc...
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

pub fn create_position_permission(
    ctx: Context<CreatePositionPermission>, 
    league: Pubkey, 
    user: Pubkey, 
    position_seq: u64,
    group_id: Pubkey,
) -> Result<()> {
    let payer = &ctx.accounts.payer;
    let position = &ctx.accounts.position;
    let permission = &mut ctx.accounts.permission;
    let group = &mut ctx.accounts.group;
    let permission_program = &ctx.accounts.permission_program;
    let system_program = &ctx.accounts.system_program;
    let members = ctx.remaining_accounts.iter().map(|acc| acc.key()).collect::<Vec<_>>();

    CreateGroupCpiBuilder::new(permission_program)
        .group(group)
        .id(group_id)
        .members(members)
        .payer(payer)
        .system_program(system_program)
        .invoke()?;

    CreatePermissionCpiBuilder::new(permission_program)
        .permission(permission)
        .delegated_account(&position.to_account_info())
        .group(group)
        .payer(payer)
        .system_program(system_program)
        .invoke_signed(&[&[
            POSITION_SEED, 
            league.key().as_ref(), 
            user.key().as_ref(), 
            position_seq.to_le_bytes().as_ref(),
            &[ctx.bumps.position],
        ]])?;

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

#[allow(unused_variables)]
pub fn open_position(
    ctx: Context<OpenPosition>, 
    position_seq: u64,
    direction: Direction,
    size: i64,
    leverage: u8,
) -> Result<()> {
    let league = &ctx.accounts.league;
    let market = &ctx.accounts.market;
    let participant = &mut ctx.accounts.participant;
    let position = &mut ctx.accounts.position;

    require!(league.status == LeagueStatus::Active, crate::errors::ErrorCode::InvalidLeagueStatus);
    require!(leverage > 0, crate::errors::ErrorCode::InvalidLeverage);
    require!(leverage <= league.max_leverage, crate::errors::ErrorCode::InvalidLeverage);    
    require!(leverage <= market.max_leverage, crate::errors::ErrorCode::InvalidLeverage);
    require!(market.price_feed == ctx.accounts.price_feed.key(), crate::errors::ErrorCode::OracleMismatch);
    require!(participant.positions.len() < 10, crate::errors::ErrorCode::MaxOpenPositionExceeded);
    require!(position.opened_at == 0, crate::errors::ErrorCode::PositionAlreadyOpened);

    let (current_price, current_exponent) = get_price_and_exponent_from_pyth(&ctx.accounts.price_feed)?;
    // For equivalent price in decimal, we need to add the quote decimals to the exponent
    let current_price_in_decimal = (current_price as f64 * 10_f64.powi(-current_exponent + QUOTE_DECIMALS as i32)).floor() as i64;
    let notional = calculate_notional(current_price_in_decimal, size, market.decimals);
    // ceil(notional / leverage) option for required margin
    let required_margin = (notional as f64 / leverage as f64).ceil() as i64;
    require!(participant.available_balance() >= required_margin, crate::errors::ErrorCode::InsufficientBalance);

    // Fill out position account
    position.direction = direction;
    position.entry_size = size;
    position.size = size;
    position.entry_price = current_price_in_decimal;
    position.notional = notional;
    position.leverage = leverage;
    position.opened_at = Clock::get()?.unix_timestamp;

    // Update participant with overflow protection
    participant.total_volume = participant
        .total_volume
        .checked_add(notional)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    participant.used_margin = participant
        .used_margin
        .checked_add(required_margin)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    participant.current_position_seq = participant
        .current_position_seq
        .checked_add(1)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    participant.positions.push(position.key());

    msg!("Position opened successfully at price {}", current_price_in_decimal);

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

/// Open Position (on ER)
/// - user open position on ER w/ price feed on ER (price feed is updated in realtime)
#[derive(Accounts)]
#[instruction(position_seq: u64)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut, 
        seeds = [
            POSITION_SEED, 
            league.key().as_ref(),
            user.key().as_ref(), 
            position_seq.to_le_bytes().as_ref()
        ], 
        bump
    )]
    pub position: Account<'info, Position>,

    #[account(
        mut,
        seeds = [
            PARTICIPANT_SEED, 
            league.key().as_ref(),
            user.key().as_ref()
        ],
        bump
    )]
    pub participant: Account<'info, Participant>,

    pub league: Account<'info, League>,
    pub market: Account<'info, Market>,
    /// CHECK: Price feed account (Pyth PriceUpdateV2)
    pub price_feed: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(league: Pubkey, user: Pubkey, position_seq: u64)]
pub struct CreatePositionPermission<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [POSITION_SEED, league.key().as_ref(), user.key().as_ref(), position_seq.to_le_bytes().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub permission: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub group: UncheckedAccount<'info>,
    /// CHECK: Checked by the permission program
    #[account(mut)]
    pub permission_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,

    // Remaining accounts include the members of the group
}
