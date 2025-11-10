use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

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
