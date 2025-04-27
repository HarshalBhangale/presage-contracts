
# Presage Prediction Market (Xion Testnet)


## A decentralized prediction market where users bet on BTC price movement (Bull/Bear).

# -----------------------------------------------------------
## 🌐 Deployment Details
# -----------------------------------------------------------

## Chain: Xion Testnet
## RPC: https://rpc.xion-testnet-2.burnt.com:443
```
Contract Address: xion12x4cy806a8349mak6dugukxkzd6426cdd6z599v3yd90d4e0n6tq9rdst7

Admin: "" (no admin, immutable)

Operator: xion19hj38fvsrmfsu94s4e3g0dk63dtflquyr5kr8k

USDC Token Address: xion1h3tgz6esvdcscx26rltzer06a2x7hamttymyxrvqkz5gak9tm97q7v60eq

Oracle Address: xion1wptw89weav8tnpgxg4fyhcahgk8yy99lka3w308536ktadkvjwxqe463hl

BTC Price Feed ID: 436ccb0d465f3cb48554bcc8def65ff695341b3ebe0897563d118b9291178d0f

```

# -----------------------------------------------------------
## 📋 How the Contract Works
# -----------------------------------------------------------

# 1. Initialization
## - Admin and Operator are set during instantiation.
## - USDC (CW20) token address must be provided for betting.
## - Oracle address is initialized (mock BTC price used for now).

# 2. Genesis Round
## - `genesis_start_round` -> starts epoch 1.
## - `genesis_lock_round` -> locks epoch 1 with initial BTC price.

# 3. Placing Bets
## - Users bet Bull (up) or Bear (down) on current epoch.
## - Must bet at least min_bet_amount (set to 1 USDC).
## - USDC is transferred from user to contract on bet.

# 4. Executing Rounds
## - `execute_round` moves to next epoch:
##   * Ends the current round.
##   * Starts a new round.
##   * Locks the round price based on mocked BTC price.

# 5. Claiming Rewards
## - Correct prediction lets users claim USDC rewards.
## - Reward = (user share from pool) - (treasury fee).

# 6. Treasury
## - Admin can claim the accumulated treasury fees.

# -----------------------------------------------------------
## 🛠️ Important Commands
# -----------------------------------------------------------

# Step 1: Instantiate Contract
xiond tx wasm instantiate $CODE_ID '{
  "admin_address": "'"$WALLET"'",
  "operator_address": "'"$WALLET"'",
  "usdc_token": "<your_cw20_token_address>",
  "oracle_address": "'"$WALLET"'",
  "btc_price_feed_id": "9d9fa0b0ecde4a7baf6b5eaa3cabe19e",
  "interval_seconds": 180,
  "buffer_seconds": 60,
  "min_bet_amount": "1000000",
  "treasury_fee": 300
}' \
  --from $WALLET --label "presage-prediction" --no-admin \
  --gas-prices 0.1uxion --gas auto --gas-adjustment 1.3 \
  --chain-id xion-testnet-2 --node https://rpc.xion-testnet-2.burnt.com:443 -y

# Step 2: Start Genesis Round
xiond tx wasm execute $CONTRACT '{"genesis_start_round":{}}' \
  --from $WALLET --gas-prices 0.1uxion --gas auto --gas-adjustment 1.3 \
  --chain-id xion-testnet-2 --node https://rpc.xion-testnet-2.burnt.com:443 -y

# Step 3: Lock Genesis Round
xiond tx wasm execute $CONTRACT '{"genesis_lock_round":{}}' \
  --from $WALLET --gas-prices 0.1uxion --gas auto --gas-adjustment 1.3 \
  --chain-id xion-testnet-2 --node https://rpc.xion-testnet-2.burnt.com:443 -y

# Step 4: Place a Bet (Example)
xiond tx wasm execute $CONTRACT '{
  "bet_bull": {
    "epoch": 1,
    "amount": "1000000"
  }
}' \
  --from $WALLET --gas-prices 0.1uxion --gas auto --gas-adjustment 1.3 \
  --chain-id xion-testnet-2 --node https://rpc.xion-testnet-2.burnt.com:443 -y

# Step 5: Query Contract Config (Example)
xiond query wasm contract-state smart $CONTRACT '{"get_config":{}}' \
  --output json --node https://rpc.xion-testnet-2.burnt.com:443


# -----------------------------------------------------------
## 🧠 Developer Notes
# -----------------------------------------------------------

## - Built using CosmWasm 1.1 style.
## - Optimized with cosmwasm/optimizer:0.16.1 Docker image.
## - Tested fully on Xion Testnet environment.

# -----------------------------------------------------------

## (This doc will be updated once mainnet deployment starts.)
