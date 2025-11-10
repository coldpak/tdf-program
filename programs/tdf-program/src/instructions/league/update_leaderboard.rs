use anchor_lang::prelude::*;

use crate::state::{Leaderboard, LEADERBOARD_SEED, Participant};

pub fn update_leaderboard_with_participant(ctx: Context<UpdateLeaderboardWithParticipant>) -> Result<()> {
    let leaderboard = &mut ctx.accounts.leaderboard;
    let participant_info = &mut ctx.accounts.participant.to_account_info();
    let mut data: &[u8] = &participant_info.try_borrow_data()?;
    let participant = Participant::try_deserialize(&mut data)?;

    let equity = participant.equity();
    let volume = participant.total_volume;

    let _ = update_topk_equity(leaderboard, participant.user, equity);
    let _ = update_topk_volume(leaderboard, participant.user, volume);

    leaderboard.last_updated = Clock::get()?.unix_timestamp;

    msg!("Updated leaderboard at {} with participant: {:?}", leaderboard.last_updated, participant.user);

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateLeaderboardWithParticipant<'info> {
    #[account(
        mut,
        seeds = [LEADERBOARD_SEED, league.key().as_ref()],
        bump
    )]
    pub leaderboard: Account<'info, Leaderboard>,

    /// CHECK: league PDA
    pub league: UncheckedAccount<'info>,
    /// CHECK: Your program ID
    pub participant: UncheckedAccount<'info>,
    // /// CHECK: the correct pda - this will be moved to the end in the future, meaning you can omit this unless needed
    // pub escrow: UncheckedAccount<'info>,
    // /// CHECK: the correct pda - this will be moved to the end in the future, meaning you can omit this unless needed
    // pub escrow_auth: UncheckedAccount<'info>,
}

fn update_topk_equity(leaderboard: &mut Leaderboard, key: Pubkey, score: i64) -> Result<()> {
  let list = &mut leaderboard.topk_equity;
  let scores = &mut leaderboard.topk_equity_scores;
  let _ = update_topk_list(list, scores, key, score, leaderboard.k);
  
  Ok(())
}

fn update_topk_volume(leaderboard: &mut Leaderboard, key: Pubkey, score: i64) -> Result<()> {
  let list = &mut leaderboard.topk_volume;
  let scores = &mut leaderboard.topk_volume_scores;
  let _ = update_topk_list(list, scores, key, score, leaderboard.k);

  Ok(())
}

fn update_topk_list(
  list: &mut Vec<Pubkey>,
  scores: &mut Vec<i64>,
  key: Pubkey,
  score: i64,
  k: u16,
) -> Result<()> {
  // If k is 0, don't do anything
  if k == 0 {
      return Ok(());
  }

  // Create a new combined vector to avoid borrowing issues
  let mut combined: Vec<(Pubkey, i64)> = Vec::new();

  // Add existing entries
  for (i, &addr) in list.iter().enumerate() {
      if i < scores.len() {
          combined.push((addr, scores[i]));
      }
  }

  // Update or add the current entry
  if let Some(pos) = combined.iter().position(|(addr, _)| *addr == key) {
      combined[pos] = (key, score);
  } else {
      combined.push((key, score));
  }

  // Sort by score (descending)
  combined.sort_by(|a, b| b.1.cmp(&a.1));

  // Keep only top k entries
  let topk = combined.into_iter().take(k as usize).collect::<Vec<_>>();

  // Update the original vectors
  list.clear();
  scores.clear();
  for (addr, sc) in topk {
      list.push(addr);
      scores.push(sc);
  }

  Ok(())
}
