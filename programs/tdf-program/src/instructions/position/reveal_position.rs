// Send SOL, update permission group members
// use anchor_lang::prelude::*;

// pub fn create_position_permission(
//   ctx: Context<CreatePositionPermission>, 
//   league: Pubkey, 
//   user: Pubkey, 
//   position_seq: u64,
//   group_id: Pubkey,
// ) -> Result<()> {
//   let payer = &ctx.accounts.payer;
//   let position = &ctx.accounts.position;
//   let permission = &mut ctx.accounts.permission;
//   let group = &mut ctx.accounts.group;
//   let permission_program = &ctx.accounts.permission_program;
//   let system_program = &ctx.accounts.system_program;
//   let members = ctx.remaining_accounts.iter().map(|acc| acc.key()).collect::<Vec<_>>();

//   CreateGroupCpiBuilder::new(permission_program)
//       .group(group)
//       .id(group_id)
//       .members(members)
//       .payer(payer)
//       .system_program(system_program)
//       .invoke()?;

//   CreatePermissionCpiBuilder::new(permission_program)
//       .permission(permission)
//       .delegated_account(&position.to_account_info())
//       .group(group)
//       .payer(payer)
//       .system_program(system_program)
//       .invoke_signed(&[&[
//           POSITION_SEED, 
//           league.key().as_ref(), 
//           user.key().as_ref(), 
//           position_seq.to_le_bytes().as_ref(),
//           &[ctx.bumps.position],
//       ]])?;

//   Ok(())
// }

