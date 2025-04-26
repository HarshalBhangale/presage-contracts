use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
    

#[cw_serde]
pub struct Config {
    pub usdc_token: Addr,
    pub admin_address: Addr,
    pub operator_address: Addr,
    pub interval_seconds: u64,
    pub buffer_seconds: u64,
    pub min_bet_amount: Uint128,
    pub treasury_fee: u64, 
    pub oracle_address: Addr, 
    pub btc_price_feed_id: String, 
}

#[cw_serde]
pub struct Round {
    pub epoch: u64,
    pub start_timestamp: u64,
    pub lock_timestamp: u64,
    pub close_timestamp: u64,
    pub lock_price: i128,
    pub close_price: i128,
    pub total_amount: Uint128,
    pub bull_amount: Uint128,
    pub bear_amount: Uint128,
    pub reward_base_amount: Uint128,
    pub reward_amount: Uint128,
    pub oracle_called: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Bull, // Price goes up
    Bear, // Price goes down
}

#[cw_serde]
pub struct BetInfo {
    pub position: Position,
    pub amount: Uint128,
    pub claimed: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PAUSED: Item<bool> = Item::new("paused");
pub const CURRENT_EPOCH: Item<u64> = Item::new("current_epoch");
pub const ROUNDS: Map<u64, Round> = Map::new("rounds"); 
pub const LEDGER: Map<(u64, Addr), BetInfo> = Map::new("ledger"); 
pub const USER_ROUNDS: Map<Addr, Vec<u64>> = Map::new("user_rounds"); 
pub const TREASURY: Item<Uint128> = Item::new("treasury");