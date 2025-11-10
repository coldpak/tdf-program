use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::state::Direction;

pub fn get_price_and_exponent_from_pyth(price_ai: &AccountInfo) -> Result<(i64, i32)> {
    // Deserialize the price feed
    let data_ref = price_ai.data.borrow();
    let price_update = PriceUpdateV2::try_deserialize_unchecked(&mut data_ref.as_ref())
        .map_err(Into::<Error>::into)?;

    // Reject if the update is older than 60 seconds : DEV
    // let maximum_age: u64 = 60;

    // Feed id is the price_update account address
    let feed_id: [u8; 32] = price_ai.key().to_bytes();

    let price = price_update.get_price_unchecked(&feed_id)?;

    Ok((price.price, price.exponent))
}

pub fn calculate_notional(price_in_decimal: i64, size: i64, decimals: u8) -> i64 {
    let scale = 10i128.pow(decimals as u32);
    let price128_in_decimal = price_in_decimal as i128;
    let size128 = size as i128;

    // (price * size) / 10^decimals
    let notional = (price128_in_decimal * size128) / scale;

    i64::try_from(notional).expect("notional overflow")
}

pub fn dir_sign(direction: Direction) -> i64 {
    match direction {
        Direction::Long => 1,
        Direction::Short => -1,
    }
}

pub fn calculate_unrealized_pnl(
    notional: i64,
    current_price: i64,
    size: i64,
    decimals: u8,
    direction: Direction,
) -> i64 {
    let current_price128 = current_price as i128;
    let size128 = size as i128;
    let scale = 10i128.pow(decimals as u32);
    let notional128 = notional as i128;
    let dir128 = dir_sign(direction.clone()) as i128;
    let pnl128 = (current_price128 * size128 / scale - notional128) * dir128;
    i64::try_from(pnl128).expect("pnl overflow")
}

pub fn calculate_price_from_notional_and_size(notional: i64, size: i64, decimals: u8) -> i64 {
    let scale = 10i128.pow(decimals as u32);
    let notional128 = notional as i128;
    let size128 = size as i128;
    let price128 = notional128 / size128 * scale;
    i64::try_from(price128).expect("price overflow")
}
