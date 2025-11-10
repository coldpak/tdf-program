use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::{delegate, commit};
use ephemeral_rollups_sdk::cpi::DelegateConfig;
use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;

use crate::state::{PARTICIPANT_SEED, Participant};

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
