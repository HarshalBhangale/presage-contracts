use cosmwasm_std::{Deps, Env};
use pyth_sdk_cw::{query_price_feed, PriceIdentifier};

use crate::error::ContractError;

pub fn get_btc_price(
    deps: Deps,
    env: Env,
    oracle_addr: String,
    price_feed_id: &str,
    max_staleness: u64,
) -> Result<i128, ContractError> {
    let price_id = match PriceIdentifier::from_hex(price_feed_id) {
        Ok(id) => id,
        Err(err) => return Err(ContractError::OracleError(format!("Invalid price feed ID: {}", err))),
    };

    let oracle_addr = deps.api.addr_validate(&oracle_addr)?;
    let price_feed_response = match query_price_feed(
        &deps.querier,
        oracle_addr,
        price_id,
    ) {
        Ok(res) => res,
        Err(e) => return Err(ContractError::OracleError(format!("Error querying price feed: {}", e))),
    };

    let price_feed = price_feed_response.price_feed;
    
    let current_time = env.block.time.seconds() as i64;
    let current_price = price_feed
        .get_price_no_older_than(current_time, max_staleness)
        .ok_or_else(|| ContractError::OracleError("Current price is not available or too stale".to_string()))?;

    Ok(i128::from(current_price.price))
}

pub fn get_mock_btc_price() -> i128 {
    90000i128
}
pub fn get_deterministic_mock_btc_price_for_testing(is_up: bool) -> i128 {
    let base_price = 50000i128;
    
    let price_change = if is_up { 1000i128 } else { -1000i128 };
    
    base_price + price_change
}