use std::error;

use chrono::{DateTime, TimeZone, Utc};
use uuid7::uuid7;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TimeStamp<T: TimeZone>(DateTime<T>);

// think of a wallet as an individual entity
#[derive(Debug, minicbor::Decode, minicbor::Encode)]
struct Wallet {
    #[n(0)]
    id: String, // uuid7 addressable string
    #[n(1)]
    address: String, // bech32 encoded
    #[n(2)]
    name: String,
    #[n(3)]
    is_active: bool,
}

// used for constructing drafts
struct TradeDetailsBuilder {
    // No ID field, as the ID *is* the hash of this struct
    trading_entity: Option<Wallet>,      // Wallet Address
    counter_party: Option<Wallet>,       // Wallet Address
    direction: Option<Direction>,        // Enum: Buy, Sell
    notional_currency: Option<Currency>, // Enum: USD, EUR, GBP
    notional_amount: u64,                // Use integers for currency
    underlying_currency: Option<Currency>,
    underlying_amount: u64,
    trade_date: Option<TimeStamp<Utc>>, // Unix timestamp
    value_date: Option<TimeStamp<Utc>>,
    delivery_date: Option<TimeStamp<Utc>>,
}

// key is the hash of this struct encoded into cbor
#[derive(minicbor::Encode, minicbor::Decode, Debug)]
struct TradeDetails {
    // No ID field, as the ID *is* the hash of this struct
    #[n(0)]
    trading_entity: Wallet, // Wallet Address
    #[n(1)]
    counter_party: Wallet, // Wallet Address
    #[n(2)]
    direction: String, // Enum: Buy, Sell
    #[n(3)]
    notional_currency: String, // Enum: USD, EUR, GBP
    #[n(4)]
    notional_amount: u64, // Use integers for currency
    #[n(5)]
    underlying_currency: String,
    #[n(6)]
    underlying_amount: u64,
    #[n(7)]
    trade_date: TimeStamp<Utc>, // Unix timestamp
    #[n(8)]
    value_date: Option<TimeStamp<Utc>>,
    #[n(9)]
    delivery_date: Option<TimeStamp<Utc>>,
}

// the state of the current trade. derived for viewing purposes.
#[derive(Debug, PartialEq)]
struct DerivedState {
    status: WitnessType,
    requester_id: String, // bech32 encoded hash of the entities uuid
    approver_id: String,
    current_details_hash: String, // hash refering to the transaction
}

#[derive(Debug, PartialEq)]
struct Witnesses {
    trade_id: String, // a hash which references the previous trade
    user_addr: String,
    timestamp_utc: TimeStamp<Utc>,
    witness_type: WitnessType,
}

#[derive(thiserror::Error, Debug)]
enum ValidationError {
    #[error("Trade Date <= Value Date <= Delivery Date failed")]
    InvalidDates,
    #[error("Trade contained a cancel witness")]
    IsCanceled,
    #[error("Trade lacks an valid approved witness")]
    NoApproved,
    #[error("Update witness was found, but no subsequent 'approve'")]
    PendingApproval,
    #[error("Trade is missing a submit witness")]
    MissingSubmit,
    #[error("Trade has already been executed and booked")]
    AlreadyExecuted,
}

#[derive(minicbor::Encode, minicbor::Decode, Debug)]
enum Currency {
    #[n(0)]
    USD,
    #[n(1)]
    GBP,
    #[n(2)]
    EUR,
}

#[derive(Debug, PartialEq)]
enum WitnessType {
    Submit {
        details_hash: String, // hash of a trade-details object
        requester_id: String,
        approver_id: String,
    },
    Approve,
    Cancel,
    Update {
        details_hash: String,
    },
    SendToExecute,
    Book {
        strike: u64,
    },
}

#[derive(minicbor::Encode, minicbor::Decode, Debug)]
enum Direction {
    #[n(0)]
    Buy,
    #[n(1)]
    Sell,
}

impl<C> minicbor::Encode<C> for TimeStamp<Utc> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.i64(self.0.timestamp())?.ok()
    }
}

impl<'b, C> minicbor::Decode<'b, C> for TimeStamp<Utc> {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        let secs = d.i64()?;

        DateTime::from_timestamp(secs, 0)
            .map(TimeStamp)
            .ok_or(minicbor::decode::Error::message(
                "failed to convert timestamp to utc",
            ))
    }
}
