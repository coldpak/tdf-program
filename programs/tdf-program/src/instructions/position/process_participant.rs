use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::{commit_accounts, commit_and_undelegate_accounts};
// Note: MagicInstructionBuilder, MagicAction, CallHandler, CommitType, ActionArgs, ShortAccountMeta
// are commented out but kept for future use in leaderboard updates

use crate::state::{LEADERBOARD_SEED, PARTICIPANT_SEED, Participant, Position};
use crate::utils::{get_price_and_exponent_from_pyth, calculate_notional, calculate_unrealized_pnl, calculate_price_from_notional_and_size};
use crate::constants::QUOTE_DECIMALS;

pub fn delegate_participant(ctx: Context<DelegateParticipant>, league: Pubkey) -> Result<()> {
    let user = &ctx.accounts.user;
    ctx.accounts.delegate_participant(
        &user,
        &[PARTICIPANT_SEED, league.as_ref(), user.key().as_ref()],
        DelegateConfig {
            validator: ctx.remaining_accounts.first().map(|acc| acc.key()),
            ..Default::default()
        },
    )?;

    msg!("Delegated participant to validator: {:?}", ctx.remaining_accounts.first().map(|acc| acc.key()));

    Ok(())
}

#[allow(unused_variables)]
pub fn undelegate_participant(ctx: Context<UndelegateParticipant>, league: Pubkey) -> Result<()> {
    commit_and_undelegate_accounts(
        &ctx.accounts.user,
        vec![&ctx.accounts.participant.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    
    msg!("Undelegated participant");
    
    Ok(())
}

/// Internal function containing the core participant update logic.
/// This is shared between `update_participant` and `update_and_commit_participant`.
fn update_participant_logic<'info>(
    participant: &mut Account<'info, Participant>,
    remaining_accounts: &[AccountInfo<'info>],
) -> Result<()> {
    let position_keys = &participant.positions.clone();

    require!(
        remaining_accounts.len() == position_keys.len() * 2,
        crate::errors::ErrorCode::InvalidUpdateParticipantRemainingAccounts
    );

    let mut total_upnl: i64 = 0;
    let mut total_used_margin: i64 = 0;
    let mut prices: Vec<i64> = Vec::new();

    // Update positions and calculate PnL
    for (i, position_key) in position_keys.iter().enumerate() {
        let position_ai = &remaining_accounts[i * 2];
        let price_feed_ai = &remaining_accounts[i * 2 + 1];

        require_keys_eq!(
            *position_key,
            position_ai.key(),
            crate::errors::ErrorCode::PositionMismatch
        );

        let mut data = position_ai.try_borrow_mut_data()?;
        let mut position: Position = Position::try_deserialize(&mut &data[..])?;

        // if position is closed, skip
        if position.size == 0 {
            prices.push(0); // placeholder for closed positions
            continue;
        }

        require_keys_eq!(
            position.price_feed,
            price_feed_ai.key(),
            crate::errors::ErrorCode::OracleMismatch
        );

        let (price, exponent) = get_price_and_exponent_from_pyth(&price_feed_ai)?;
        let price_in_decimal = (price as f64 * 10_f64.powi(-exponent + QUOTE_DECIMALS as i32)).floor() as i64;
        prices.push(price_in_decimal);

        let new_upnl = calculate_unrealized_pnl(
            position.notional,
            price_in_decimal,
            position.size,
            position.market_decimals,
            position.direction.clone(),
        );

        position.unrealized_pnl = new_upnl;

        let mut dst = &mut data[..];
        position.try_serialize(&mut dst)?;

        total_upnl = total_upnl
            .checked_add(new_upnl)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;

        let margin_for_pos = position
            .notional
            .checked_div(position.leverage as i64)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;
        total_used_margin = total_used_margin
            .checked_add(margin_for_pos)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;
    }

    participant.unrealized_pnl = total_upnl;
    participant.used_margin = total_used_margin;

    msg!(
        "Participant updated: unrealized_pnl: {}, used_margin: {}, equity: {}",
        total_upnl,
        total_used_margin,
        participant.equity()
    );

    // Handle liquidation if equity is negative
    if participant.equity() < 0 {
        liquidate_participant_positions(participant, position_keys, remaining_accounts, &prices)?;
    }

    Ok(())
}

/// Internal function to handle liquidation of all positions when equity is negative.
fn liquidate_participant_positions<'info>(
    participant: &mut Account<'info, Participant>,
    position_keys: &[Pubkey],
    remaining_accounts: &[AccountInfo<'info>],
    prices: &[i64],
) -> Result<()> {
    msg!("💥 Auto liquidation triggered");

    for (i, position_key) in position_keys.iter().enumerate() {
        let position_ai = &remaining_accounts[i * 2];

        let mut data = position_ai.try_borrow_mut_data()?;
        let mut position: Position = Position::try_deserialize(&mut &data[..])?;

        if position.size == 0 {
            continue;
        }

        let price = prices[i];
        let realized_pnl = position.unrealized_pnl;
        let released_margin = position
            .notional
            .checked_div(position.leverage as i64)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;
        let closing_equity = calculate_notional(price, position.size, position.market_decimals);

        // Calculate closed stats with overflow protection
        position.closed_size = position
            .closed_size
            .checked_add(position.size)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;

        position.closed_equity = position
            .closed_equity
            .checked_add(closing_equity)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;

        // Safe division for closed_price
        if position.closed_size > 0 {
            position.closed_price = calculate_price_from_notional_and_size(
                position.closed_equity,
                position.closed_size,
                position.market_decimals,
            );
        }

        position.closed_pnl = position
            .closed_pnl
            .checked_add(realized_pnl)
            .ok_or(crate::errors::ErrorCode::MathOverflow)?;

        // Update position
        position.size = 0;
        position.notional = 0;
        position.unrealized_pnl = 0;
        position.closed_at = Clock::get()?.unix_timestamp;

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

        msg!(
            "Position liquidated: {} (realized_pnl: {}, released_margin: {})",
            position_key,
            realized_pnl,
            released_margin
        );

        let mut dst = &mut data[..];
        position.try_serialize(&mut dst)?;
    }

    // Clear all positions after liquidation
    participant.positions.clear();
    participant.unrealized_pnl = 0;

    msg!("All positions liquidated. Participant equity reset.");
    Ok(())
}

/// Updates participant without committing accounts.
/// Use this when you only need to update the participant state on the ephemeral rollup.
#[allow(unused_variables)]
pub fn update_participant<'info>(
    ctx: Context<'_, '_, 'info, 'info, UpdateParticipant<'info>>,
    league: Pubkey,
    user: Pubkey,
) -> Result<()> {
    update_participant_logic(&mut ctx.accounts.participant, ctx.remaining_accounts)
}

#[allow(unused_variables)]
pub fn commit_participant(ctx: Context<UpdateParticipant>, league: Pubkey, user: Pubkey) -> Result<()> {
    commit_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.participant.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    Ok(())
}

/// Updates participant and commits accounts to the main chain.
/// Use this when you need to persist the participant state changes.
#[allow(unused_variables)]
pub fn update_and_commit_participant<'info>(
    ctx: Context<'_, '_, 'info, 'info, UpdateParticipant<'info>>, 
    league: Pubkey,
    user: Pubkey,
) -> Result<()> {
    // Update participant on ER
    update_participant_logic(&mut ctx.accounts.participant, ctx.remaining_accounts)?;

    // Collect accounts to commit (participant + all positions)
    let participant_account_info = ctx.accounts.participant.to_account_info();
    let mut committing_accounts: Vec<&AccountInfo<'info>> = vec![&participant_account_info];

    let position_count = ctx.accounts.participant.positions.len();
    for i in 0..position_count {
        let position_ai = &ctx.remaining_accounts[i * 2];
        committing_accounts.push(position_ai);
    }

    // Commit accounts
    commit_accounts(
        &ctx.accounts.payer, 
        committing_accounts, 
        &ctx.accounts.magic_context.to_account_info(), 
        &ctx.accounts.magic_program.to_account_info(),
    )?;

    // Update leaderboard - Magic Actions
    // let instruction_data = anchor_lang::InstructionData::data(
    //     &crate::instructions::UpdateLeaderboardWithParticipant {}
    // );

    // let action_args = ActionArgs {
    //     escrow_index: 0,
    //     data: instruction_data,
    // };

    // let accounts = vec![
    //     ShortAccountMeta {
    //         pubkey: ctx.accounts.leaderboard.key(),
    //         is_writable: true,
    //     },
    //     ShortAccountMeta {
    //         pubkey: league.key(),
    //         is_writable: false,
    //     },
    //     ShortAccountMeta {
    //         pubkey: ctx.accounts.participant.key(),
    //         is_writable: false,
    //     },
    // ];

    // let call_handler = CallHandler {
    //     args: action_args,
    //     compute_units: 200_000,
    //     escrow_authority: ctx.accounts.payer.to_account_info(),
    //     destination_program: crate::ID,
    //     accounts,
    // };

    // let magic_builder = MagicInstructionBuilder {
    //     payer: ctx.accounts.payer.to_account_info(),
    //     magic_context: ctx.accounts.magic_context.to_account_info(),
    //     magic_program: ctx.accounts.magic_program.to_account_info(),
    //     magic_action: MagicAction::Commit(CommitType::WithHandler {
    //         commited_accounts: vec![ctx.accounts.leaderboard.to_account_info()],
    //         call_handlers: vec![call_handler],
    //     }),
    // };

    // magic_builder.build_and_invoke()?;
    
    Ok(())
}

#[delegate]
#[derive(Accounts)]
pub struct DelegateParticipant<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Participant account
    #[account(mut, del)]
    pub participant: AccountInfo<'info>,
}

#[commit]
#[derive(Accounts)]
#[instruction(league: Pubkey)]
pub struct UndelegateParticipant<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut, seeds = [
            PARTICIPANT_SEED, 
            league.as_ref(), 
            user.key().as_ref()
        ], 
        bump
    )]
    pub participant: Account<'info, Participant>,
}

#[commit]
#[derive(Accounts)]
#[instruction(league: Pubkey, user: Pubkey)]
pub struct UpdateParticipant<'info> {
    #[account(
        mut,
        seeds = [PARTICIPANT_SEED, league.as_ref(), user.as_ref()],
        bump
    )]
    pub participant: Account<'info, Participant>,

    /// CHECK: Leaderboard PDA - not mut here, writable set in handler
    #[account(
        seeds = [LEADERBOARD_SEED, league.as_ref()],
        bump
    )]
    pub leaderboard: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: your program ID
    pub program_id: AccountInfo<'info>,

    // Remaining accounts:
    // [position_index_0, price_feed_index_0, position_index_1, price_feed_index_1, ...]
}
