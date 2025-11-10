use anchor_lang::prelude::*;

use crate::state::{GlobalConfig, Market, MARKET_SEED, MARKET_SPACE};

pub fn create_market(
  ctx: Context<CreateMarket>,
  symbol: [u8; 16],
  decimals: u8,
  max_leverage: u8,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;

    let market = &mut ctx.accounts.market;

    market.symbol = symbol;
    market.price_feed = ctx.accounts.price_feed.key();
    market.decimals = decimals;
    market.max_leverage = max_leverage;
    market.listed_by = ctx.accounts.admin.key();
    market.created_at = now;
    market.is_active = true;

    market.bump = ctx.bumps.market;

    Ok(())
}

pub fn update_market(
  ctx: Context<UpdateMarket>,
  symbol: [u8; 16],
  decimals: u8,
  is_active: bool,
  max_leverage: u8,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.symbol = symbol;
    market.decimals = decimals;
    market.is_active = is_active;
    market.max_leverage = max_leverage;

    Ok(())
}

pub fn delete_market(_ctx: Context<DeleteMarket>) -> Result<()> {
    // Account will be closed automatically by Anchor's close constraint
    msg!("Market account deleted");
    Ok(())
}


#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = admin,
        space = MARKET_SPACE, 
        seeds = [MARKET_SEED, price_feed.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    /// CHECK: Price feed account
    pub price_feed: AccountInfo<'info>,

    #[account(mut)]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
      mut,
      constraint = admin.key() == global_config.admin
    )]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMarket<'info> {
    #[account(mut, seeds = [MARKET_SEED, price_feed.key().as_ref()], bump)]
    pub market: Account<'info, Market>,

    /// CHECK: Price feed account
    pub price_feed: AccountInfo<'info>,

    #[account(mut)]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
      mut,
      constraint = admin.key() == global_config.admin
    )]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DeleteMarket<'info> {
    #[account(
        mut,
        close = admin,
        seeds = [MARKET_SEED, price_feed.key().as_ref()],
        bump
    )]
    pub market: Account<'info, Market>,

    /// CHECK: Price feed account
    pub price_feed: AccountInfo<'info>,

    #[account(mut)]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
      mut,
      constraint = admin.key() == global_config.admin
    )]
    pub admin: Signer<'info>,   
    pub system_program: Program<'info, System>,
}
