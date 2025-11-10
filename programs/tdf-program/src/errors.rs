use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid market length")]
    InvalidMarketLength,
    #[msg("Invalid reward vault")]
    InvalidRewardVault,
    #[msg("Invalid time range")]
    InvalidTimeRange,
    #[msg("Invalid k value")]
    InvalidKValue,
    #[msg("Invalid league status")]
    InvalidLeagueStatus,
    #[msg("Not a league creator")]
    NotLeagueCreator,
    #[msg("Invalid leverage")]
    InvalidLeverage,
    #[msg("Max open position exceeded")]
    MaxOpenPositionExceeded,
    #[msg("Invalid position sequence")]
    InvalidPositionSequence,
    #[msg("Oracle mismatch")]
    OracleMismatch,
    #[msg("Invalid oracle price feed data")]
    InvalidOraclePriceFeed,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Insufficient balance")]
    InsufficientBalance,
    #[msg("Position already opened")]
    PositionAlreadyOpened,
    #[msg("Invalid update participant remaining accounts")]
    InvalidUpdateParticipantRemainingAccounts,
    #[msg("Position mismatch")]
    PositionMismatch,
}
