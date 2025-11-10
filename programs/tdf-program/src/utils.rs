use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

pub fn get_price_from_pyth(price_ai: &AccountInfo) -> Result<i64> {
    // Deserialize the price feed
    let data_ref = price_ai.data.borrow();
    let price_update = PriceUpdateV2::try_deserialize_unchecked(&mut data_ref.as_ref())
        .map_err(Into::<Error>::into)?;

    // Reject if the update is older than 60 seconds : DEV
    // let maximum_age: u64 = 60;

    // Feed id is the price_update account address
    let feed_id: [u8; 32] = price_ai.key().to_bytes();

    let price = price_update.get_price_unchecked(&feed_id)?;

    msg!(
        "The price is ({} ± {}) * 10^-{}",
        price.price,
        price.conf,
        price.exponent
    );
    msg!(
        "The price is: {}",
        price.price as f64 * 10_f64.powi(-price.exponent)
    );
    msg!("Slot: {}", price_update.posted_slot);
    msg!("Message: {:?}", price_update.price_message);

    Ok(price.price)
}

pub fn calculate_notional(price: i64, size: i64, decimals: u8) -> i64 {
    let scale = 10i128.pow(decimals as u32);
    let price128 = price as i128;
    let size128 = size as i128;

    // (price * size) / 10^decimals
    let notional = (price128 * size128) / scale;

    i64::try_from(notional).expect("notional overflow")
}
