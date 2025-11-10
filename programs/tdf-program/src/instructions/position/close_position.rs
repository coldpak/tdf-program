use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::ephem::{commit_accounts, commit_and_undelegate_accounts};

use crate::state::{Position, PARTICIPANT_SEED, POSITION_SEED, League, Market, Participant, LeagueStatus};
use crate::utils::{get_price_and_exponent_from_pyth, calculate_notional, dir_sign, calculate_price_from_notional_and_size, calculate_unrealized_pnl};
use crate::constants::QUOTE_DECIMALS;

pub fn close_position(ctx: Context<ClosePosition>, position_seq: u64) -> Result<()> {
    let league = &ctx.accounts.league;
    let market = &ctx.accounts.market;
    let participant = &mut ctx.accounts.participant;
    let position = &mut ctx.accounts.position;

    require!(league.status == LeagueStatus::Active, crate::errors::ErrorCode::InvalidLeagueStatus);
    require!(position.opened_at != 0, crate::errors::ErrorCode::PositionNotOpened);
    require!(position.closed_at == 0, crate::errors::ErrorCode::PositionAlreadyClosed);
    require!(market.price_feed == ctx.accounts.price_feed.key(), crate::errors::ErrorCode::OracleMismatch);

    let (current_price, current_exponent) = get_price_and_exponent_from_pyth(&ctx.accounts.price_feed)?;
    let current_price_in_decimal = (current_price as f64 * 10_f64.powi(-current_exponent + QUOTE_DECIMALS as i32)).floor() as i64;

    let prev_upnl = position.unrealized_pnl;

    // Calculate realized PnL with overflow protection
    let closing_size = position.size;
    let closing_equity = calculate_notional(current_price_in_decimal, closing_size, market.decimals);
    let closing_notional = calculate_notional(position.entry_price, closing_size, market.decimals);
    let realized_pnl = (closing_equity as i64 - closing_notional as i64)
        .checked_mul(dir_sign(position.direction.clone()) as i64)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    let prev_locked = position
        .notional
        .checked_div(position.leverage as i64)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    let new_locked = position
        .notional
        .checked_sub(closing_notional)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?
        .checked_div(position.leverage as i64)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    let released_margin = prev_locked
        .checked_sub(new_locked)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;

    // Calculate closed stats with overflow protection
    position.closed_size = position
        .closed_size
        .checked_add(closing_size)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    position.closed_equity = position
        .closed_equity
        .checked_add(closing_equity)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    position.closed_price = calculate_price_from_notional_and_size(
        position.closed_equity,
        position.closed_size,
        market.decimals,
    );
    position.closed_pnl = position
        .closed_pnl
        .checked_add(realized_pnl)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;

    // Update position with overflow protection
    position.size = position
        .size
        .checked_sub(closing_size)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    position.notional = position
        .notional
        .checked_sub(closing_notional)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    position.unrealized_pnl = calculate_unrealized_pnl(
        position.notional,
        current_price,
        position.size,
        market.decimals,
        position.direction.clone(),
    );

    // Update participant with overflow protection
    participant.total_volume = participant
        .total_volume
        .checked_add(closing_equity)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    participant.used_margin = participant
        .used_margin
        .checked_sub(released_margin)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;

    participant.virtual_balance = participant
        .virtual_balance
        .checked_add(realized_pnl)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;

    let upnl_delta = position
        .unrealized_pnl
        .checked_sub(prev_upnl)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    participant.unrealized_pnl = participant
        .unrealized_pnl
        .checked_add(upnl_delta)
        .ok_or(crate::errors::ErrorCode::MathOverflow)?;

    // close position logic here
    position.closed_at = Clock::get()?.unix_timestamp;
    // remove position from participant.positions vector
    participant.positions.retain(|p| p != &position.key());
    
    commit_accounts(
        &ctx.accounts.user,
        vec![&participant.to_account_info(), &position.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    msg!("Position closed and removed from participant, commit requested");

    Ok(())
}

#[commit]
#[derive(Accounts)]
#[instruction(position_seq: u64)]
pub struct ClosePosition<'info> {
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
