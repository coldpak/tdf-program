use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::delegate;
use ephemeral_rollups_sdk::cpi::DelegateConfig;

use magicblock_permission_client::instructions::{
    CreateGroupCpiBuilder, CreatePermissionCpiBuilder,
};

use crate::state::{
    PrivateResourceExample, PRIVATE_RESOURCE_EXAMPLE_SEED, PRIVATE_RESOURCE_EXAMPLE_SPACE,
    GlobalConfig,
};

pub fn create_private_resource_example(ctx: Context<CreatePrivateResourceExample>) -> Result<()> {
    let private_resource_example = &mut ctx.accounts.private_resource_example;
    private_resource_example.bump = ctx.bumps.private_resource_example;

    Ok(())
}

pub fn delegate_private_resource_example(ctx: Context<DelegatePrivateResourceExample>, participant: Pubkey) -> Result<()> {
    ctx.accounts.delegate_pda(
        &ctx.accounts.user,
        &[PRIVATE_RESOURCE_EXAMPLE_SEED, participant.key().as_ref()],
        DelegateConfig {
            ..Default::default()
        },
    )?;

    Ok(())
}

pub fn update_private_resource_example(ctx: Context<UpdatePrivateResourceExample>, value: String) -> Result<()> {
    let private_resource_example = &mut ctx.accounts.private_resource_example;
    private_resource_example.value = value;

    Ok(())
}

pub fn create_example_permission(
  ctx: Context<CreateExamplePermission>, 
  user: Pubkey,
) -> Result<()> {
    let payer = &ctx.accounts.admin;
    let private_resource_example = &ctx.accounts.private_resource_example;
    let permission = &mut ctx.accounts.permission;
    let group = &mut ctx.accounts.group;
    let permission_program = &ctx.accounts.permission_program;
    let system_program = &ctx.accounts.system_program;
    let members = ctx.remaining_accounts.iter().map(|acc| acc.key()).collect::<Vec<_>>();

    CreateGroupCpiBuilder::new(permission_program)
        .group(group)
        .id(user)
        .members(members)
        .payer(payer)
        .system_program(system_program)
        .invoke()?;

    CreatePermissionCpiBuilder::new(permission_program)
        .permission(permission)
        .delegated_account(&private_resource_example.to_account_info())
        .group(group)
        .payer(payer)
        .system_program(system_program)
        .invoke_signed(&[&[
            PRIVATE_RESOURCE_EXAMPLE_SEED,
            user.key().as_ref(),
            &[ctx.bumps.private_resource_example],
        ]])?;

    Ok(())
}

#[derive(Accounts)]
pub struct CreatePrivateResourceExample<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = PRIVATE_RESOURCE_EXAMPLE_SPACE,
        seeds = [PRIVATE_RESOURCE_EXAMPLE_SEED, user.key().as_ref()],
        bump
    )]
    pub private_resource_example: Account<'info, PrivateResourceExample>,

    pub system_program: Program<'info, System>,
}

#[delegate]
#[derive(Accounts)]
pub struct DelegatePrivateResourceExample<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    /// CHECK: Private resource example account
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct UpdatePrivateResourceExample<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [PRIVATE_RESOURCE_EXAMPLE_SEED, user.key().as_ref()], bump)]
    pub private_resource_example: Account<'info, PrivateResourceExample>,
}

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct CreateExamplePermission<'info> {
    #[account(
      mut,
      constraint = admin.key() == global_config.admin
    )]
    pub admin: Signer<'info>,

    #[account(mut, seeds = [PRIVATE_RESOURCE_EXAMPLE_SEED, user.key().as_ref()], bump)]
    pub private_resource_example: Account<'info, PrivateResourceExample>,

    /// CHECK: Global config account
    pub global_config: Account<'info, GlobalConfig>,

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
