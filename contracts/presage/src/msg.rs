use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ExecuteMsg;


use crate::state::Position;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin_address: String,
    pub operator_address: String,
    pub usdc_token: String,
    pub interval_seconds: u64,
    pub buffer_seconds: u64,
    pub min_bet_amount: Uint128,
    pub treasury_fee: u64, 
    pub oracle_address: String, // Pyth oracle address
    pub btc_price_feed_id: String, // Pyth price feed ID for BTC/USD
}

#[cw_serde]
pub enum ExecuteMsg {
    // User actions
    BetBull { epoch: u64, amount: Uint128 },
    BetBear { epoch: u64, amount: Uint128 },
    Claim { epochs: Vec<u64> },
    
    // Operator actions
    ExecuteRound {},
    GenesisStartRound {},
    GenesisLockRound {},
    
    // Admin actions
    Pause {},
    Unpause {},
    ClaimTreasury {},
    SetBufferAndIntervalSeconds { buffer_seconds: u64, interval_seconds: u64 },
    SetMinBetAmount { min_bet_amount: Uint128 },
    SetOperator { operator_address: String },
    SetTreasuryFee { treasury_fee: u64 },
    SetOracleInfo { oracle_address: String, btc_price_feed_id: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(RoundResponse)]
    GetRound { epoch: u64 },
    
    #[returns(u64)]
    GetCurrentEpoch {},
    
    #[returns(UserRoundsResponse)]
    GetUserRounds { user: String, cursor: u64, size: u64 },
    
    #[returns(ClaimableResponse)]
    Claimable { epoch: u64, user: String },
    
    #[returns(RefundableResponse)]
    Refundable { epoch: u64, user: String },
    
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct RoundResponse {
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

#[cw_serde]
pub struct UserRoundsResponse {
    pub epochs: Vec<u64>,
    pub next_cursor: Option<u64>,
}

#[cw_serde]
pub struct ClaimableResponse {
    pub is_claimable: bool,
    pub position: Option<Position>,
    pub amount: Option<Uint128>,
    pub expected_reward: Option<Uint128>,
}

#[cw_serde]
pub struct RefundableResponse {
    pub is_refundable: bool,
    pub amount: Option<Uint128>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub usdc_token: String,
    pub admin_address: String,
    pub operator_address: String,
    pub interval_seconds: u64,
    pub buffer_seconds: u64,
    pub min_bet_amount: Uint128,
    pub treasury_fee: u64,
    pub oracle_address: String,
    pub btc_price_feed_id: String,
    pub paused: bool,
}