use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod constants;
mod errors;
mod instructions;
mod state;
mod utils;

declare_id!("V1fxrKvUB7ebNyhe8R7tYiPLYSNsicWwowyY6pbYrxM");

#[ephemeral]
#[program]
pub mod tdf_program {
    pub use super::instructions::*;
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize(ctx, fee_bps)
    }

    pub fn update_treasury(ctx: Context<UpdateTreasury>) -> Result<()> {
        instructions::update_treasury(ctx)
    }

    pub fn update_admin(ctx: Context<UpdateAdmin>) -> Result<()> {
        instructions::update_admin(ctx)
    }

    pub fn update_fee_bps(ctx: Context<UpdateFeeBps>, new_fee_bps: u16) -> Result<()> {
        instructions::update_fee_bps(ctx, new_fee_bps)
    }

    pub fn create_market(
        ctx: Context<CreateMarket>,
        symbol: [u8; 16],
        decimals: u8,
        max_leverage: u8,
    ) -> Result<()> {
        instructions::create_market(ctx, symbol, decimals, max_leverage)
    }

    pub fn update_market(
        ctx: Context<UpdateMarket>,
        symbol: [u8; 16],
        decimals: u8,
        is_active: bool,
        max_leverage: u8,
    ) -> Result<()> {
        instructions::update_market(ctx, symbol, decimals, is_active, max_leverage)
    }

    pub fn delete_market(ctx: Context<DeleteMarket>) -> Result<()> {
        instructions::delete_market(ctx)
    }

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
        instructions::create_league(
            ctx,
            id,
            markets,
            entry_amount,
            virtual_on_deposit,
            start_ts,
            end_ts,
            metadata_uri,
            max_participants,
            max_leverage,
            k,
        )
    }

    pub fn start_league(ctx: Context<StartLeague>) -> Result<()> {
        instructions::start_league(ctx)
    }

    pub fn close_league(ctx: Context<CloseLeague>) -> Result<()> {
        instructions::close_league(ctx)
    }

    pub fn join_league(ctx: Context<JoinLeague>) -> Result<()> {
        instructions::join_league(ctx)
    }

    pub fn delegate_participant(ctx: Context<DelegateParticipant>, league: Pubkey) -> Result<()> {
        instructions::delegate_participant(ctx, league)
    }

    pub fn undelegate_participant(
        ctx: Context<UndelegateParticipant>,
        league: Pubkey,
    ) -> Result<()> {
        instructions::undelegate_participant(ctx, league)
    }

    pub fn init_unopened_position(
        ctx: Context<InitUnopenedPosition>,
        league: Pubkey,
        current_position_seq: u64,
        market: Pubkey,
        market_decimals: u8,
    ) -> Result<()> {
        instructions::init_unopened_position(
            ctx,
            league,
            current_position_seq,
            market,
            market_decimals,
        )
    }

    pub fn create_position_permission(
        ctx: Context<CreatePositionPermission>,
        league: Pubkey,
        user: Pubkey,
        position_seq: u64,
        group_id: Pubkey,
    ) -> Result<()> {
        instructions::create_position_permission(ctx, league, user, position_seq, group_id)
    }

    pub fn delegate_unopened_position(
        ctx: Context<DelegateUnopenedPosition>,
        participant: Pubkey,
        position_seq: u64,
    ) -> Result<()> {
        instructions::delegate_unopened_position(ctx, participant, position_seq)
    }

    pub fn open_position(
        ctx: Context<OpenPosition>,
        position_seq: u64,
        direction: crate::state::Direction,
        size: i64,
        leverage: u8,
    ) -> Result<()> {
        instructions::open_position(ctx, position_seq, direction, size, leverage)
    }

    pub fn close_position(ctx: Context<ClosePosition>, position_seq: u64) -> Result<()> {
        instructions::close_position(ctx, position_seq)
    }

    pub fn commit_position(
        ctx: Context<CommitPosition>,
        league: Pubkey,
        user: Pubkey,
        position_seq: u64,
    ) -> Result<()> {
        instructions::commit_position(ctx, league, user, position_seq)
    }

    pub fn update_participant<'info>(
        ctx: Context<'_, '_, 'info, 'info, UpdateParticipant<'info>>,
        league: Pubkey,
        user: Pubkey,
    ) -> Result<()> {
        instructions::update_participant(ctx, league, user)
    }

    pub fn commit_participant(
        ctx: Context<UpdateParticipant>,
        league: Pubkey,
        user: Pubkey,
    ) -> Result<()> {
        instructions::commit_participant(ctx, league, user)
    }

    pub fn update_and_commit_participant<'info>(
        ctx: Context<'_, '_, 'info, 'info, UpdateParticipant<'info>>,
        league: Pubkey,
        user: Pubkey,
    ) -> Result<()> {
        instructions::update_and_commit_participant(ctx, league, user)
    }

    pub fn update_leaderboard_with_participant(
        ctx: Context<UpdateLeaderboardWithParticipant>,
    ) -> Result<()> {
        instructions::update_leaderboard_with_participant(ctx)
    }
}
