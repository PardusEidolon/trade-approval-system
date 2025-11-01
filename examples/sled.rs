#![allow(warnings)]

use bech32::Bech32;
use chrono::{DateTime, TimeZone, Utc};
use sled::Db;
use std::{str::FromStr, time};
use uuid7;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TimeStamp<T: TimeZone>(DateTime<T>);

impl<C> minicbor::Encode<C> for TimeStamp<Utc> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.i64(self.0.timestamp())?.ok()
    }
}

impl<'b, C> minicbor::Decode<'b, C> for TimeStamp<Utc> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        let secs = d.i64()?;

        DateTime::from_timestamp(secs, 0)
            .map(TimeStamp)
            .ok_or(minicbor::decode::Error::message(
                "failed to convert timestamp to utc",
            ))
    }
}

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

impl TradeDetails {
    fn new() -> Self {
        let trading_entity = Wallet::new(String::new());
        let counter_party = Wallet::new(String::new());
        let direction = "buy".to_string();
        let notional_currency = "GBP".to_string();
        let notional_amount = 0;
        let underlying_currency = "USD".to_string();
        let underlying_amount = 0;
        let trade_date = TimeStamp(Utc::now());
        let value_date = None;
        let delivery_date = None;

        Self {
            trading_entity,
            counter_party,
            direction,
            notional_currency,
            notional_amount,
            underlying_currency,
            underlying_amount,
            trade_date,
            value_date,
            delivery_date,
        }
    }
    fn to_cbor(&self) -> Vec<u8> {
        minicbor::to_vec(self).unwrap()
    }
    fn hash_trade(&self) -> Vec<u8> {
        sha256::digest(self.to_cbor()).as_bytes().to_vec()
    }
    // We first encode the transaction into Cbor then hash it; this then becomes our key pointing to the encoded transaction.
    fn insert_into_db(&self, db: &Db) -> Option<String> {
        let hash = self.hash_trade();
        let cbor = self.to_cbor();

        db.insert(hash.as_slice(), cbor.as_slice());

        match String::from_utf8(hash) {
            Ok(str) => Some(str),
            Err(_) => None,
        }
    }
    fn get_self(&self, db: &Db) -> Option<Self> {
        if let Some(vec) = db.get(self.hash_trade()).unwrap() {
            let datum: TradeDetails =
                minicbor::decode(vec.as_ref()).expect("failed to decode cbor to TradeDetails Type");

            return Some(datum);
        }
        None
    }

    fn insert_value_date(&mut self, date: TimeStamp<Utc>) {
        self.value_date = Some(date);
    }
    fn insert_delivery_date(&mut self, date: TimeStamp<Utc>) {
        self.delivery_date = Some(date);
    }
}

impl Wallet {
    fn new(name: String) -> Self {
        let uuid = uuid7::uuid7();
        let id = sha256::digest(uuid.as_bytes());

        let hrp = bech32::Hrp::parse_unchecked("addr_");
        let address = bech32::encode::<Bech32>(hrp, id.as_bytes()).unwrap();

        let is_active = false;

        Self {
            id,
            address,
            name,
            is_active,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let db = sled::open("sled")?;

    if !db.is_empty() {
        db.clear();
    }

    let trade = TradeDetails::new();

    trade.insert_into_db(&db);

    if let Some(val) = trade.get_self(&db) {
        println!("{:#?}", val);
    }

    Ok(())
}
