//! Validation and operational error types
use chrono::Utc;

use super::trade::TimeStamp;

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error(
        "dates failed to meet the following condition: trade_date <= value_date <= delivery_date"
    )]
    DateValidation,
    #[error("Trade contained a cancel witness")]
    IsCanceled,
    #[error("Trade is missing an valid approved witness")]
    NoApproved,
    #[error("Update witness was found, but no subsequent 'approve'")]
    PendingApproval,
    #[error("Trade is missing a submit witness")]
    MissingSubmit,
    #[error("Trade has already been executed and booked")]
    AlreadyExecuted,
}

#[derive(thiserror::Error, Debug)]
pub enum TradeError {
    #[error("Invalid Date: `{0}` is `{1:?}`")]
    InvalidDate(String, Option<TimeStamp<Utc>>),
    #[error("Malformed Entity: `{0:?}`")]
    InvalidEntity(Option<String>),
    #[error("Currency Ticker does not exist")]
    InvalidCurrency,
}
