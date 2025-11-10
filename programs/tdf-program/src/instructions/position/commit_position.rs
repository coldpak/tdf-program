use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::commit;
use ephemeral_rollups_sdk::ephem::commit_accounts;

use crate::state::{Position, POSITION_SEED};

#[allow(unused_variables)]
pub fn commit_position(ctx: Context<CommitPosition>, league: Pubkey, user: Pubkey, position_seq: u64) -> Result<()> {
    commit_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.position.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;

    Ok(())
}

#[commit]
#[derive(Accounts)]
#[instruction(league: Pubkey, user: Pubkey, position_seq: u64)]
pub struct CommitPosition<'info> {
  #[account(mut)]
  pub payer: Signer<'info>,

  #[account(mut, seeds = [POSITION_SEED, league.key().as_ref(), user.key().as_ref(), position_seq.to_le_bytes().as_ref()], bump)]
  pub position: Account<'info, Position>,
}
