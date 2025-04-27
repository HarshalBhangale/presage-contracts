use cosmwasm_std::{StdError, OverflowError, DivideByZeroError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid interval seconds")]
    InvalidInterval {},

    #[error("Invalid buffer seconds (must be less than interval)")]
    InvalidBuffer {},

    #[error("Invalid minimum bet amount")]
    InvalidMinBetAmount {},

    #[error("Invalid treasury fee (must be <= 1000, representing max 10%)")]
    InvalidTreasuryFee {},

    #[error("Contract is paused")]
    Paused {},

    #[error("Genesis round has not been started")]
    GenesisNotStarted {},

    #[error("Genesis round has already been started")]
    GenesisAlreadyStarted {},

    #[error("Round is not bettable")]
    RoundNotBettable {},

    #[error("Already bet on this round")]
    AlreadyBet {},

    #[error("Bet amount is too small")]
    BetTooSmall {},

    #[error("Round has not ended for epoch {epoch}")]
    RoundNotEnded { epoch: u64 },

    #[error("No bet record found for epoch {epoch}")]
    NoBetRecord { epoch: u64 },

    #[error("Already claimed rewards for epoch {epoch}")]
    AlreadyClaimed { epoch: u64 },

    #[error("Not a winner for epoch {epoch}")]
    NotWinner { epoch: u64 },

    #[error("No epochs provided")]
    EmptyEpochs {},

    #[error("Contract is already paused")]
    AlreadyPaused {},

    #[error("Contract is already unpaused")]
    AlreadyUnpaused {},

    #[error("No treasury funds to claim")]
    NoTreasury {},

    #[error("Oracle error: {0}")]
    OracleError(String),


    #[error("invalid usdc token")]
    InvalidBetFunds
}