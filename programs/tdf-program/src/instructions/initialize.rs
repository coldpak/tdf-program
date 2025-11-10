use anchor_lang::prelude::*;

use crate::state::{GlobalConfig, GLOBAL_CONFIG_SEED, GLOBAL_CONFIG_SPACE};

pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    global_config.admin = ctx.accounts.admin.key();
    global_config.fee_bps = fee_bps;
    global_config.treasury = ctx.accounts.treasury.key();
    global_config.bump = ctx.bumps.global_config;

    Ok(())
}

pub fn update_treasury(ctx: Context<UpdateTreasury>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    global_config.treasury = ctx.accounts.new_treasury.key();

    Ok(())
}

pub fn update_admin(ctx: Context<UpdateAdmin>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    global_config.admin = ctx.accounts.new_admin.key();

    Ok(())
}

pub fn update_fee_bps(ctx: Context<UpdateFeeBps>, new_fee_bps: u16) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    global_config.fee_bps = new_fee_bps;
    
    Ok(())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
      init,
      payer = admin,
      space = GLOBAL_CONFIG_SPACE,
      seeds = [GLOBAL_CONFIG_SEED],
      bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK:
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateTreasury<'info> {
    #[account(mut, seeds = [GLOBAL_CONFIG_SEED], bump)]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK:
    pub new_treasury: AccountInfo<'info>,

    #[account(mut, constraint = admin.key() == global_config.admin)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateAdmin<'info> {
    #[account(mut, seeds = [GLOBAL_CONFIG_SEED], bump)]
    pub global_config: Account<'info, GlobalConfig>,

    /// CHECK:
    pub new_admin: AccountInfo<'info>,

    #[account(mut, constraint = admin.key() == global_config.admin)]
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateFeeBps<'info> {
    #[account(mut, seeds = [GLOBAL_CONFIG_SEED], bump)]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut, constraint = admin.key() == global_config.admin)]
    pub admin: Signer<'info>,
}
