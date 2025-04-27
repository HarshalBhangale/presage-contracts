#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo,
    Response, StdResult, WasmMsg, SubMsg, Uint128,
};
use cw20::Cw20ExecuteMsg;

use cw2::set_contract_version;
use pyth_sdk_cw::PriceIdentifier;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, QueryMsg, RoundResponse, ConfigResponse, 
    UserRoundsResponse, ClaimableResponse, RefundableResponse,
};
use crate::state::{
    Config, Round, Position, BetInfo, ROUNDS, LEDGER, USER_ROUNDS, 
    CONFIG, CURRENT_EPOCH, PAUSED, TREASURY,
};
use crate::oracle::{get_btc_price,get_mock_btc_price};


const CONTRACT_NAME: &str = "crates.io:presage-prediction";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Oracle constants
const ORACLE_TIME_LIMIT: u64 = 60; // 60 seconds staleness limit

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Validate addresses
    let admin_address = deps.api.addr_validate(&msg.admin_address)?;
    let operator_address = deps.api.addr_validate(&msg.operator_address)?;
    let usdc_token = deps.api.addr_validate(&msg.usdc_token)?;
    let oracle_address = deps.api.addr_validate(&msg.oracle_address)?;

    // Validate parameters
    if msg.interval_seconds == 0 {
        return Err(ContractError::InvalidInterval {});
    }
    if msg.buffer_seconds >= msg.interval_seconds {
        return Err(ContractError::InvalidBuffer {});
    }
    if msg.min_bet_amount == Uint128::zero() {
        return Err(ContractError::InvalidMinBetAmount {});
    }
    if msg.treasury_fee > 1000 {
        return Err(ContractError::InvalidTreasuryFee {});
    }

    let config = Config {
        usdc_token,
        admin_address,
        operator_address,
        interval_seconds: msg.interval_seconds,
        buffer_seconds: msg.buffer_seconds,
        min_bet_amount: msg.min_bet_amount,
        treasury_fee: msg.treasury_fee,
        oracle_address,
        btc_price_feed_id: msg.btc_price_feed_id.clone(), // Clone to fix the moved value error
    };

    CONFIG.save(deps.storage, &config)?;
    CURRENT_EPOCH.save(deps.storage, &0u64)?;
    PAUSED.save(deps.storage, &false)?;
    TREASURY.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin_address)
        .add_attribute("operator", msg.operator_address)
        .add_attribute("usdc_token", msg.usdc_token)
        .add_attribute("oracle_address", msg.oracle_address)
        .add_attribute("btc_price_feed_id", msg.btc_price_feed_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::BetBull { epoch, amount } => execute_bet(deps, env, info, epoch, amount, Position::Bull),
        ExecuteMsg::BetBear { epoch, amount } => execute_bet(deps, env, info, epoch, amount, Position::Bear),
        ExecuteMsg::Claim { epochs } => execute_claim(deps, env, info, epochs),
        ExecuteMsg::ExecuteRound {} => execute_round(deps, env, info),
        ExecuteMsg::GenesisStartRound {} => execute_genesis_start_round(deps, env, info),
        ExecuteMsg::GenesisLockRound {} => execute_genesis_lock_round(deps, env, info),
        ExecuteMsg::Pause {} => execute_pause(deps, info),
        ExecuteMsg::Unpause {} => execute_unpause(deps, info),
        ExecuteMsg::ClaimTreasury {} => execute_claim_treasury(deps, env, info),
        ExecuteMsg::SetBufferAndIntervalSeconds { buffer_seconds, interval_seconds } => 
            execute_set_buffer_and_interval_seconds(deps, info, buffer_seconds, interval_seconds),
        ExecuteMsg::SetMinBetAmount { min_bet_amount } => 
            execute_set_min_bet_amount(deps, info, min_bet_amount),
        ExecuteMsg::SetOperator { operator_address } => 
            execute_set_operator(deps, info, operator_address),
        ExecuteMsg::SetTreasuryFee { treasury_fee } => 
            execute_set_treasury_fee(deps, info, treasury_fee),
        ExecuteMsg::SetOracleInfo { oracle_address, btc_price_feed_id } =>
            execute_set_oracle_info(deps, info, oracle_address, btc_price_feed_id),
    }
}

fn execute_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    epoch: u64,
    amount: Uint128,
    position: Position,
) -> Result<Response, ContractError> {
    let paused = PAUSED.load(deps.storage)?;
    if paused {
        return Err(ContractError::Paused {});
    }

    let config = CONFIG.load(deps.storage)?;
    
    if amount < config.min_bet_amount {
        return Err(ContractError::BetTooSmall {});
    }

    let mut round = ROUNDS.load(deps.storage, epoch)?;
    if env.block.time.seconds() >= round.lock_timestamp {
        return Err(ContractError::RoundNotBettable {});
    }

    let user_addr = info.sender.clone();
    if LEDGER.has(deps.storage, (epoch, user_addr.clone())) {
        return Err(ContractError::AlreadyBet {});
    }
    let sent_amount = info.funds.iter().find(|c| c.denom == config.usdc_token).map(|c| c.amount).unwrap_or(Uint128::zero());

    if sent_amount != amount {
        return Err(ContractError::InvalidBetFunds {});
    }

    // let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
    //     contract_addr: config.usdc_token.to_string(),
    //     msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
    //         owner: user_addr.to_string(),
    //         recipient: env.contract.address.to_string(),
    //         amount, 
    //     })?,
    //     funds: vec![],
    // });

    match position {
        Position::Bull => round.bull_amount += amount,
        Position::Bear => round.bear_amount += amount,
    }
    round.total_amount += amount;
    ROUNDS.save(deps.storage, epoch, &round)?;

    let bet_info = BetInfo {
        position: position.clone(),
        amount,
        claimed: false,
    };



    LEDGER.save(deps.storage, (epoch, user_addr.clone()), &bet_info)?;

    let mut user_rounds = USER_ROUNDS.may_load(deps.storage, user_addr.clone())?.unwrap_or_default();
    if !user_rounds.contains(&epoch) {
        user_rounds.push(epoch);
        USER_ROUNDS.save(deps.storage, user_addr, &user_rounds)?;
    }

    let position_str = match position {
        Position::Bull => "bull",
        Position::Bear => "bear",
    };
    
    Ok(Response::new()
        .add_attribute("method", "bet")
        .add_attribute("position", position_str)
        .add_attribute("user", info.sender)
        .add_attribute("epoch", epoch.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    epochs: Vec<u64>,
) -> Result<Response, ContractError> {
    if epochs.is_empty() {
        return Err(ContractError::EmptyEpochs {});
    }

    let config = CONFIG.load(deps.storage)?;
    let user_addr = info.sender.clone();
    let mut total_reward = Uint128::zero();
    let mut events = Vec::new();

    for epoch in epochs.iter() {
        let round = ROUNDS.load(deps.storage, *epoch)?;
        if !round.oracle_called {
            return Err(ContractError::RoundNotEnded { epoch: *epoch });
        }

        if !LEDGER.has(deps.storage, (*epoch, user_addr.clone())) {
            return Err(ContractError::NoBetRecord { epoch: *epoch });
        }

        let mut bet_info = LEDGER.load(deps.storage, (*epoch, user_addr.clone()))?;
        if bet_info.claimed {
            return Err(ContractError::AlreadyClaimed { epoch: *epoch });
        }

        let reward = calculate_reward(round.clone(), bet_info.clone())?;
        if reward == Uint128::zero() {
            return Err(ContractError::NotWinner { epoch: *epoch });
        }

        bet_info.claimed = true;
        LEDGER.save(deps.storage, (*epoch, user_addr.clone()), &bet_info)?;

        total_reward += reward;

        events.push(Event::new("claim")
            .add_attribute("epoch", epoch.to_string())
            .add_attribute("user", user_addr.to_string())
            .add_attribute("reward", reward.to_string()));
    }

    let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.usdc_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: user_addr.to_string(),
            amount: total_reward, 
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_submessage(SubMsg::new(transfer_msg))
        .add_attribute("method", "claim")
        .add_attribute("user", user_addr)
        .add_attribute("total_reward", total_reward.to_string())
        .add_events(events))
}

fn calculate_reward(round: Round, bet_info: BetInfo) -> Result<Uint128, ContractError> {
    let is_winner = match bet_info.position {
        Position::Bull => round.close_price > round.lock_price,
        Position::Bear => round.close_price < round.lock_price,
    };

    if !is_winner || round.close_price == round.lock_price {
        return Ok(Uint128::zero());
    }

    let position_amount = match bet_info.position {
        Position::Bull => round.bull_amount,
        Position::Bear => round.bear_amount,
    };

    let opposing_amount = match bet_info.position {
        Position::Bull => round.bear_amount,
        Position::Bear => round.bull_amount,
    };

    if position_amount == Uint128::zero() {
        return Ok(Uint128::zero());
    }

    let reward_amount = if opposing_amount == Uint128::zero() {
        bet_info.amount
    } else {
        let reward_base = round.total_amount.checked_sub(round.reward_base_amount)?;
        let reward = reward_base.checked_mul(bet_info.amount)?.checked_div(position_amount)?;
        reward
    };

    Ok(reward_amount)
}

fn execute_round(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.operator_address {
        return Err(ContractError::Unauthorized {});
    }

    let paused = PAUSED.load(deps.storage)?;
    if paused {
        return Err(ContractError::Paused {});
    }

    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;
    if current_epoch == 0 {
        return Err(ContractError::GenesisNotStarted {});
    }

    let current_round = ROUNDS.load(deps.storage, current_epoch)?;
    let current_timestamp = env.block.time.seconds();

    if current_timestamp >= current_round.close_timestamp {
        execute_end_round(&mut deps, env.clone(), current_epoch, &config)?;

        let new_epoch = current_epoch + 1;
        execute_start_round(&mut deps, env.clone(), new_epoch, &config)?;

        let response = execute_lock_round(&mut deps, env, new_epoch, &config)?;
        
        return Ok(response);
    } else if current_timestamp >= current_round.lock_timestamp {
        execute_lock_round(&mut deps, env.clone(), current_epoch, &config)?;

        let response = execute_start_round(&mut deps, env, current_epoch + 1, &config)?;
        
        return Ok(response);
    }

    Ok(Response::new()
        .add_attribute("method", "execute_round")
        .add_attribute("action", "no_action_needed")
        .add_attribute("current_epoch", current_epoch.to_string())
        .add_attribute("current_timestamp", current_timestamp.to_string()))
}

fn execute_start_round(
    deps: &mut DepsMut,
    env: Env,
    epoch: u64,
    config: &Config,
) -> Result<Response, ContractError> {
    let start_timestamp = env.block.time.seconds();
    let lock_timestamp = start_timestamp + config.interval_seconds - config.buffer_seconds;
    let close_timestamp = start_timestamp + config.interval_seconds;

    let new_round = Round {
        epoch,
        start_timestamp,
        lock_timestamp,
        close_timestamp,
        lock_price: 0,
        close_price: 0,
        total_amount: Uint128::zero(),
        bull_amount: Uint128::zero(),
        bear_amount: Uint128::zero(),
        reward_base_amount: Uint128::zero(),
        reward_amount: Uint128::zero(),
        oracle_called: false,
    };

    ROUNDS.save(deps.storage, epoch, &new_round)?;
    CURRENT_EPOCH.save(deps.storage, &epoch)?;

    let event = Event::new("start_round")
        .add_attribute("epoch", epoch.to_string())
        .add_attribute("start_timestamp", start_timestamp.to_string())
        .add_attribute("lock_timestamp", lock_timestamp.to_string())
        .add_attribute("close_timestamp", close_timestamp.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_attribute("method", "start_round")
        .add_attribute("epoch", epoch.to_string()))
}

fn execute_lock_round(
    deps: &mut DepsMut,
    env: Env,
    epoch: u64,
    config: &Config,
) -> Result<Response, ContractError> {
    // Get current BTC price from Pyth oracle
    let oracle_addr = config.oracle_address.to_string();
    let price_feed_id = config.btc_price_feed_id.clone();
    /* 
    let lock_price = get_btc_price(
        deps.as_ref(),
        env.clone(),
        oracle_addr,
        &price_feed_id,
        ORACLE_TIME_LIMIT,
    )?;  */

    let lock_price = get_mock_btc_price();


    let mut round = ROUNDS.load(deps.storage, epoch)?;
    round.lock_price = lock_price;
    
    let treasury_fee = round.total_amount * Uint128::from(config.treasury_fee) / Uint128::from(10000u32);
    round.reward_base_amount = treasury_fee;
    
    let mut treasury = TREASURY.load(deps.storage)?;
    treasury += treasury_fee;
    TREASURY.save(deps.storage, &treasury)?;
    
    ROUNDS.save(deps.storage, epoch, &round)?;

    let event = Event::new("lock_round")
        .add_attribute("epoch", epoch.to_string())
        .add_attribute("lock_timestamp", env.block.time.seconds().to_string())
        .add_attribute("lock_price", lock_price.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_attribute("method", "lock_round")
        .add_attribute("epoch", epoch.to_string()))
}

fn execute_end_round(
    deps: &mut DepsMut,
    env: Env,
    epoch: u64,
    config: &Config,
) -> Result<Response, ContractError> {
    // Get current BTC price from Pyth oracle
    let oracle_addr = config.oracle_address.to_string();
    let price_feed_id = config.btc_price_feed_id.clone();
    /* 
    let close_price = get_btc_price(
        deps.as_ref(),
        env.clone(),
        oracle_addr,
        &price_feed_id,
        ORACLE_TIME_LIMIT,
    )?;
    */

    let close_price = get_mock_btc_price();

    let mut round = ROUNDS.load(deps.storage, epoch)?;
    round.close_price = close_price;
    round.oracle_called = true;
    ROUNDS.save(deps.storage, epoch, &round)?;

    let event = Event::new("end_round")
        .add_attribute("epoch", epoch.to_string())
        .add_attribute("close_timestamp", env.block.time.seconds().to_string())
        .add_attribute("close_price", close_price.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_attribute("method", "end_round")
        .add_attribute("epoch", epoch.to_string()))
}

fn execute_genesis_start_round(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.operator_address {
        return Err(ContractError::Unauthorized {});
    }

    let paused = PAUSED.load(deps.storage)?;
    if paused {
        return Err(ContractError::Paused {});
    }

    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;
    if current_epoch != 0 {
        return Err(ContractError::GenesisAlreadyStarted {});
    }

    let response = execute_start_round(&mut deps, env, 1, &config)?;

    Ok(response)
}

fn execute_genesis_lock_round(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.operator_address {
        return Err(ContractError::Unauthorized {});
    }

    let paused = PAUSED.load(deps.storage)?;
    if paused {
        return Err(ContractError::Paused {});
    }

    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;
    if current_epoch != 1 {
        return Err(ContractError::GenesisNotStarted {});
    }

    let response = execute_lock_round(&mut deps, env, 1, &config)?;

    Ok(response)
}

fn execute_pause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let paused = PAUSED.load(deps.storage)?;
    if paused {
        return Err(ContractError::AlreadyPaused {});
    }

    PAUSED.save(deps.storage, &true)?;

    Ok(Response::new()
        .add_attribute("method", "pause")
        .add_attribute("admin", info.sender))
}

fn execute_unpause(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let paused = PAUSED.load(deps.storage)?;
    if !paused {
        return Err(ContractError::AlreadyUnpaused {});
    }

    PAUSED.save(deps.storage, &false)?;

    Ok(Response::new()
        .add_attribute("method", "unpause")
        .add_attribute("admin", info.sender))
}

fn execute_claim_treasury(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let treasury = TREASURY.load(deps.storage)?;
    if treasury == Uint128::zero() {
        return Err(ContractError::NoTreasury {});
    }

    TREASURY.save(deps.storage, &Uint128::zero())?;

    let transfer_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.usdc_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: config.admin_address.to_string(),
            amount: treasury, 
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_submessage(SubMsg::new(transfer_msg))
        .add_attribute("method", "claim_treasury")
        .add_attribute("admin", info.sender)
        .add_attribute("amount", treasury.to_string()))
}

fn execute_set_buffer_and_interval_seconds(
    deps: DepsMut,
    info: MessageInfo,
    buffer_seconds: u64,
    interval_seconds: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    if interval_seconds == 0 {
        return Err(ContractError::InvalidInterval {});
    }
    if buffer_seconds >= interval_seconds {
        return Err(ContractError::InvalidBuffer {});
    }

    config.interval_seconds = interval_seconds;
    config.buffer_seconds = buffer_seconds;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "set_buffer_and_interval_seconds")
        .add_attribute("buffer_seconds", buffer_seconds.to_string())
        .add_attribute("interval_seconds", interval_seconds.to_string()))
}

fn execute_set_min_bet_amount(
    deps: DepsMut,
    info: MessageInfo,
    min_bet_amount: Uint128,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    if min_bet_amount == Uint128::zero() {
        return Err(ContractError::InvalidMinBetAmount {});
    }

    config.min_bet_amount = min_bet_amount;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "set_min_bet_amount")
        .add_attribute("min_bet_amount", min_bet_amount.to_string()))
}

fn execute_set_operator(
    deps: DepsMut,
    info: MessageInfo,
    operator_address: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let operator_addr = deps.api.addr_validate(&operator_address)?;

    config.operator_address = operator_addr;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "set_operator")
        .add_attribute("operator", operator_address))
}

fn execute_set_treasury_fee(
    deps: DepsMut,
    info: MessageInfo,
    treasury_fee: u64,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    if treasury_fee > 1000 {
        return Err(ContractError::InvalidTreasuryFee {});
    }

    config.treasury_fee = treasury_fee;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "set_treasury_fee")
        .add_attribute("treasury_fee", treasury_fee.to_string()))
}

fn execute_set_oracle_info(
    deps: DepsMut,
    info: MessageInfo,
    oracle_address: String,
    btc_price_feed_id: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let oracle_addr = deps.api.addr_validate(&oracle_address)?;

    if PriceIdentifier::from_hex(&btc_price_feed_id).is_err() {
        return Err(ContractError::OracleError("Invalid price feed ID format".to_string()));
    }

    config.oracle_address = oracle_addr;
    config.btc_price_feed_id = btc_price_feed_id.clone();
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "set_oracle_info")
        .add_attribute("oracle_address", oracle_address)
        .add_attribute("btc_price_feed_id", btc_price_feed_id))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetRound { epoch } => to_json_binary(&query_round(deps, epoch)?),
        QueryMsg::GetCurrentEpoch {} => to_json_binary(&query_current_epoch(deps)?),
        QueryMsg::GetUserRounds { user, cursor, size } => to_json_binary(&query_user_rounds(deps, user, cursor, size)?),
        QueryMsg::Claimable { epoch, user } => to_json_binary(&query_claimable(deps, epoch, user)?),
        QueryMsg::Refundable { epoch, user } => to_json_binary(&query_refundable(deps, epoch, user)?),
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_round(deps: Deps, epoch: u64) -> StdResult<RoundResponse> {
    let round = ROUNDS.load(deps.storage, epoch)?;
    Ok(RoundResponse {
        epoch: round.epoch,
        start_timestamp: round.start_timestamp,
        lock_timestamp: round.lock_timestamp,
        close_timestamp: round.close_timestamp,
        lock_price: round.lock_price,
        close_price: round.close_price,
        total_amount: round.total_amount,
        bull_amount: round.bull_amount,
        bear_amount: round.bear_amount,
        reward_base_amount: round.reward_base_amount,
        reward_amount: round.reward_amount,
        oracle_called: round.oracle_called,
    })
}

fn query_current_epoch(deps: Deps) -> StdResult<u64> {
    let current_epoch = CURRENT_EPOCH.load(deps.storage)?;
    Ok(current_epoch)
}
fn query_user_rounds(
    deps: Deps,
    user: String,
    cursor: u64,
    size: u64,
) -> StdResult<UserRoundsResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    let user_rounds = USER_ROUNDS.may_load(deps.storage, user_addr)?.unwrap_or_default();
    
    let start = if cursor == 0 { 0 } else { 
        let pos = user_rounds.iter().position(|&x| x == cursor);
        match pos {
            Some(idx) => if idx + 1 < user_rounds.len() { idx + 1 } else { user_rounds.len() },
            None => 0,
        }
    };
    
    let end = std::cmp::min(start + size as usize, user_rounds.len());
    let result: Vec<u64> = user_rounds[start..end].to_vec();
    
    let next_cursor = if end < user_rounds.len() { Some(user_rounds[end]) } else { None };
    
    Ok(UserRoundsResponse {
        epochs: result,
        next_cursor,
    })
}

// Query function to check if a round is claimable for a user
fn query_claimable(
    deps: Deps,
    epoch: u64,
    user: String,
) -> StdResult<ClaimableResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    let round = ROUNDS.load(deps.storage, epoch)?;
    if !round.oracle_called {
        return Ok(ClaimableResponse {
            is_claimable: false,
            position: None,
            amount: None,
            expected_reward: None,
        });
    }
    if !LEDGER.has(deps.storage, (epoch, user_addr.clone())) {
        return Ok(ClaimableResponse {
            is_claimable: false,
            position: None,
            amount: None,
            expected_reward: None,
        });
    }
    
    let bet_info = LEDGER.load(deps.storage, (epoch, user_addr))?;
    if bet_info.claimed {
        return Ok(ClaimableResponse {
            is_claimable: false,
            position: Some(bet_info.position),
            amount: Some(bet_info.amount),
            expected_reward: None,
        });
    }
    
    let reward = match calculate_reward(round, bet_info.clone()) {
        Ok(amount) => amount,
        Err(_) => Uint128::zero(),
    };
    
    let is_claimable = reward > Uint128::zero();
    
    Ok(ClaimableResponse {
        is_claimable,
        position: Some(bet_info.position),
        amount: Some(bet_info.amount),
        expected_reward: if is_claimable { Some(reward) } else { None },
    })
}

fn query_refundable(
    deps: Deps,
    epoch: u64,
    user: String,
) -> StdResult<RefundableResponse> {
    let user_addr = deps.api.addr_validate(&user)?;
    
    let round = ROUNDS.load(deps.storage, epoch)?;
    
    if !LEDGER.has(deps.storage, (epoch, user_addr.clone())) {
        return Ok(RefundableResponse {
            is_refundable: false,
            amount: None,
        });
    }
    
    let bet_info = LEDGER.load(deps.storage, (epoch, user_addr))?;
    if bet_info.claimed {
        return Ok(RefundableResponse {
            is_refundable: false,
            amount: None,
        });
    }
    
    let is_refundable = round.oracle_called && round.lock_price == round.close_price;
    
    Ok(RefundableResponse {
        is_refundable,
        amount: if is_refundable { Some(bet_info.amount) } else { None },
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    let paused = PAUSED.load(deps.storage)?;
    
    Ok(ConfigResponse {
        usdc_token: config.usdc_token.to_string(),
        admin_address: config.admin_address.to_string(),
        operator_address: config.operator_address.to_string(),
        interval_seconds: config.interval_seconds,
        buffer_seconds: config.buffer_seconds,
        min_bet_amount: config.min_bet_amount,
        treasury_fee: config.treasury_fee,
        oracle_address: config.oracle_address.to_string(),
        btc_price_feed_id: config.btc_price_feed_id,
        paused,
    })
}