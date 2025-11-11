use anchor_lang::prelude::*;

#[account]
pub struct Counter {
    pub count: u64,
}

#[account]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub fee_bps: u16, // e.g., 1000 = 10%
    pub treasury: Pubkey,

    pub bump: u8,
}

pub const GLOBAL_CONFIG_SEED: &[u8] = b"global_config";
pub const GLOBAL_CONFIG_SPACE: usize = 8 + 32 + 2 + 32 + 1;

#[account]
pub struct Market {
    pub symbol: [u8; 16],   // e.g., "SOL/USDC"
    pub price_feed: Pubkey, // price oracle address
    pub decimals: u8, // e.g., 8 for SOLUSD by pyth
    pub is_active: bool,
    pub max_leverage: u8, // e.g., 20
    
    // metadata
    pub listed_by: Pubkey, // admin
    pub created_at: i64,

    pub bump: u8,
}

pub const MARKET_SEED: &[u8] = b"market";
pub const MARKET_SPACE: usize = 8 + 16 + 32 + 1 + 1 + 1 + 32 + 8 + 1;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum LeagueStatus {
    Pending,
    Active,
    Closed,
}

#[account]
pub struct League {
    pub id: String, // String with max length of 32 bytes
    pub creator: Pubkey,
    pub status: LeagueStatus,
    pub markets: Vec<Pubkey>, // list of market addresses, max length of 10
    pub leaderboard: Pubkey,  // leaderboard address

    // entry fee, rewards
    pub entry_token_mint: Pubkey, // SPL token for entry fees, if SOL => wSOL
    pub entry_amount: i64,        // token amount to enter the league
    pub reward_vault: Pubkey, // SPL token vault for rewards. TODO: add RewardVault account struct for more tokens
    pub total_reward_amount: u64, // Total reward amount fixed at close_league
    pub virtual_on_deposit: i64, // Paper dollar (e.g., 10_000 * 1e6)

    // metadata
    pub metadata_uri: String, // URI to the league metadata
    pub start_ts: i64,        // timestamp
    pub end_ts: i64,          // timestamp
    pub max_participants: u32,
    pub max_leverage: u8, // e.g. 20x

    pub bump: u8,
}

pub const LEAGUE_SEED: &[u8] = b"league";
pub const LEAGUE_SPACE: usize = 8
    + (4 + 32)
    + 32
    + 1
    + (4 + 32 * 10)
    + 32
    + 32
    + 8
    + 32
    + 8
    + 8
    + (4 + 200)
    + 8
    + 8
    + 4
    + 1
    + 1;

#[account]
pub struct Leaderboard {
    pub league: Pubkey,
    pub k: u16,                       // top k participants, max is 10 for now
    pub topk_equity: Vec<Pubkey>,     // participant pubkeys
    pub topk_equity_scores: Vec<i64>, // scores of top k participants

    pub topk_volume: Vec<Pubkey>,     // participant pubkeys
    pub topk_volume_scores: Vec<i64>, // scores of top k participants

    pub last_updated: i64,
    pub bump: u8,
}

pub const LEADERBOARD_SEED: &[u8] = b"leaderboard";
pub const LEADERBOARD_SPACE: usize =
    8 + 32 + 2 + (4 + 32 * 10) + (4 + 8 * 10) + (4 + 32 * 10) + (4 + 8 * 10) + 8 + 1;

#[account]
pub struct Participant {
    pub league: Pubkey,
    pub user: Pubkey,
    pub claimed: bool, // if the user has claimed the reward

    // Realtime stats
    pub virtual_balance: i64, // Paper dollar (e.g., 10_000 * 1e6), only update when position is updated
    pub unrealized_pnl: i64,  // accumulated unrealized PnL, update with position checking cycle
    pub used_margin: i64, // used margin for current position, update with position is opened or updated

    pub total_volume: i64, // accumulated volume, only update when position is opened or updated
    pub topk_equity_index: u16, // TopK equity index if not in, 0xFFFF
    pub topk_volume_index: u16, // TopK volume index if not in, 0xFFFF

    // Position tracking sequence number
    pub current_position_seq: u64, // sequence number of current position
    pub positions: Vec<Pubkey>,    // position accounts, max length is 10

    pub bump: u8,
}
impl Participant {
    // equity = virtual_balance + unrealized_pnl
    // available balance = equity - used_margin
    pub fn equity(&self) -> i64 {
        self.virtual_balance + self.unrealized_pnl
    }

    pub fn available_balance(&self) -> i64 {
        self.equity() - self.used_margin
    }
}

pub const PARTICIPANT_SEED: &[u8] = b"participant";
pub const PARTICIPANT_SPACE: usize =
    8 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 2 + 2 + 8 + (4 + 32 * 10) + 1;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum Direction {
    Long = 1,
    Short = -1,
}

#[account]
pub struct Position {
    pub league: Pubkey,
    pub user: Pubkey,
    pub market: Pubkey,
    pub market_decimals: u8,
    pub price_feed: Pubkey,
    pub seq_num: u64, // sequence number for position tracking

    pub direction: Direction,
    pub entry_price: i64, // average price in price-decimal (1e6)
    pub entry_size: i64,  // token amount of entry size
    pub leverage: u8,     // e.g. 5x

    // Realtime stats
    pub size: i64,           // token amount of current position
    pub notional: i64,       // cache: entry_price * size (1e6) - input capital in $
    pub unrealized_pnl: i64, // (last_updated_price - entry_price) * size * direction
    // notional + unrealized_pnl = current value of position in $
    pub opened_at: i64,
    pub closed_at: i64,

    pub closed_size: i64,   // size closed so far
    pub closed_price: i64,  // price in price-decimal (1e6)
    pub closed_equity: i64, // closed_price * size (1e6)
    pub closed_pnl: i64,    // (closed_notional - notional) * direction

    pub bump: u8,
}

pub const POSITION_SEED: &[u8] = b"position";
pub const POSITION_SPACE: usize =
    8 + (32 + 32 + 32 + 1 + 32 + 8) + (1 + 8 + 8 + 1) + 8 * 5 + 8 * 4 + 1;

#[account]
pub struct PrivateResourceExample {
    pub value: String,

    pub bump: u8,
}

pub const PRIVATE_RESOURCE_EXAMPLE_SEED: &[u8] = b"private_resource_example";
pub const PRIVATE_RESOURCE_EXAMPLE_SPACE: usize = 8 + (4 + 2000) + 1;
