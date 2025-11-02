use super::error::{TradeError, ValidationError};
use chrono::{DateTime, TimeZone, Utc};
use uuid7::{Uuid, uuid7};

#[derive(minicbor::Encode, minicbor::Decode, Debug)]
pub enum Currency {
    #[n(0)]
    USD,
    #[n(1)]
    GBP,
    #[n(2)]
    EUR,
}

#[derive(minicbor::Encode, minicbor::Decode, Debug)]
pub enum Direction {
    #[n(0)]
    Buy,
    #[n(1)]
    Sell,
}

// also used for constructing drafts
// key is the hash of this struct encoded into cbor
#[derive(minicbor::Encode, minicbor::Decode, Debug, Default)]
pub struct TradeDetails {
    // No ID field, as the ID *is* the hash of this struct
    #[n(0)]
    trading_entity: Option<EntityID>, // Wallet Address
    #[n(1)]
    counter_party: Option<EntityID>, // Wallet Address
    #[n(2)]
    direction: Option<Direction>,
    #[n(3)]
    notional_currency: Option<Currency>,
    #[n(4)]
    notional_amount: u64,
    #[n(5)]
    underlying_currency: Option<Currency>,
    #[n(6)]
    underlying_amount: u64,
    #[n(7)]
    trade_date: Option<TimeStamp<Utc>>,
    #[n(8)]
    value_date: Option<TimeStamp<Utc>>,
    #[n(9)]
    delivery_date: Option<TimeStamp<Utc>>,
    #[n(10)]
    strike: Option<u64>, // The agreed upon rate
}

// newtype wrapper over uuid because Uuid doesn't implement minicbor traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserID(Uuid);

#[derive(Debug, minicbor::Decode, minicbor::Encode)]
#[cbor(array)]
pub struct EntityID(#[n(0)] UserID); // uuid7 type string

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct TimeStamp<T: TimeZone>(DateTime<T>);

impl UserID {
    pub fn new() -> Self {
        Self(uuid7())
    }
}

impl TimeStamp<Utc> {
    pub fn new() -> Self {
        Self(Utc::now())
    }
    pub fn new_with(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> Self {
        Utc.with_ymd_and_hms(year, month, day, hour, min, sec)
            .unwrap()
            .into()
    }
    pub fn to_datetime_utc(&self) -> DateTime<Utc> {
        self.0
    }
}

impl EntityID {
    pub fn new() -> Self {
        Self(UserID::new())
    }
}

impl TradeDetails {
    /// Construct a new builder object, this becomes the basis for a draft
    pub fn new() -> Self {
        Self::default()
    }
    pub fn new_trade_entity(mut self, trade_entity: EntityID) -> Self {
        self.trading_entity = Some(trade_entity);
        self
    }
    pub fn new_counter_party(mut self, counter_party: EntityID) -> Self {
        self.counter_party = Some(counter_party);
        self
    }
    pub fn set_direction(mut self, direction: Direction) -> Self {
        self.direction = Some(direction);
        self
    }
    pub fn set_notional_currency(mut self, symbol: Currency) -> Self {
        self.notional_currency = Some(symbol);
        self
    }
    pub fn set_underlying_currency(mut self, symbol: Currency) -> Self {
        self.underlying_currency = Some(symbol);
        self
    }
    pub fn set_notional_amount(mut self, amount: u64) -> Self {
        self.notional_amount = amount;
        self
    }
    pub fn set_underlying_amount(mut self, amount: u64) -> Self {
        self.underlying_amount = amount;
        self
    }
    pub fn set_trade_date(mut self, date: TimeStamp<Utc>) -> Self {
        self.trade_date = Some(date);
        self
    }
    pub fn set_value_date(mut self, date: TimeStamp<Utc>) -> Self {
        self.value_date = Some(date);
        self
    }
    pub fn set_delivery_date(mut self, date: TimeStamp<Utc>) -> Self {
        self.delivery_date = Some(date);
        self
    }
    pub fn set_strike(mut self, rate: u64) -> Self {
        self.strike = Some(rate);
        self
    }
    /// Checks if the predicate `a <= b <= c` is true as referenced in the exercise doc
    pub fn validate_dates(&self) -> bool {
        let a = self.trade_date.as_ref();
        let b = self.value_date.as_ref();
        let c = self.delivery_date.as_ref();

        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => {
                let trade_date = a.to_datetime_utc();
                let value_date = b.to_datetime_utc();
                let delivery_date = c.to_datetime_utc();

                trade_date <= value_date && value_date <= delivery_date
            }
            _ => false,
        }
    }
    // Checks fields, and performs validation. returns a hash of the trade and its contetents serialised into cbor
    pub fn build(&self) -> anyhow::Result<(String, Vec<u8>)> {
        if self.trading_entity.is_none() {
            return Err(TradeError::InvalidEntity(self.trading_entity).into());
        }
        if self.counter_party.is_none() {
            return Err(TradeError::InvalidEntity(self.counter_party).into());
        }
        if self.direction.is_none() {
            return Err(anyhow::Error::msg("Direction is not set"));
        }
        if self.notional_currency.is_none() {
            return Err(TradeError::InvalidCurrency.into());
        }
        if self.notional_amount == 0 {
            return Err(anyhow::Error::msg("Notional amount is set to zero"));
        }
        if self.underlying_currency.is_none() {
            return Err(TradeError::InvalidCurrency.into());
        }
        if self.underlying_amount == 0 {
            return Err(anyhow::Error::msg("underlying amount is set to zero"));
        }

        if self.trade_date.is_none() {
            return Err(TradeError::InvalidDate("Trade Date".into(), self.trade_date).into());
        }
        if self.value_date.is_none() {
            return Err(TradeError::InvalidDate("value Date".into(), self.value_date).into());
        }
        if self.delivery_date.is_none() {
            return Err(TradeError::InvalidDate("Delivery Date".into(), self.trade_date).into());
        }
        if !self.validate_dates() {
            return Err(ValidationError::DateValidation.into());
        }

        let contents = minicbor::to_vec(self)?;
        let hash = sha256::digest(&contents);

        Ok((hash, contents))
    }
}
impl<T: TimeZone> From<DateTime<T>> for TimeStamp<T> {
    fn from(value: DateTime<T>) -> Self {
        TimeStamp(value)
    }
}
impl<C> minicbor::Encode<C> for TimeStamp<Utc> {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        _: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        if let Some(nsec) = self.0.timestamp_nanos_opt() {
            return e.i64(nsec)?.ok();
        }

        Err(minicbor::encode::Error::message(
            "failed to encode timestamp. timestamp_nanos_opt returned None",
        ))
    }
}
impl<'b, C> minicbor::Decode<'b, C> for TimeStamp<Utc> {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        let nsecs = d.i64()?;

        Ok(TimeStamp(DateTime::from_timestamp_nanos(nsecs)))
    }
}
impl<C> minicbor::Encode<C> for UserID {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        self.0.as_bytes().encode(e, ctx)
    }
}
impl<'b, C> minicbor::Decode<'b, C> for UserID {
    fn decode(d: &mut minicbor::Decoder<'b>, _: &mut C) -> Result<Self, minicbor::decode::Error> {
        let digest: [u8; 16] = d.decode()?;

        Ok(UserID(Uuid::from(digest)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn userid_encoding() {
        let original = UserID::new();

        let encoding = minicbor::to_vec(original.clone()).unwrap();
        let decode: UserID = minicbor::decode(&encoding).unwrap();

        assert_eq!(original, decode);
    }
    #[test]
    fn timestamp_encoding() {
        let original = TimeStamp::new();

        let encoding = minicbor::to_vec(original.clone()).unwrap();
        let decode: TimeStamp<Utc> = minicbor::decode(&encoding).unwrap();

        assert_eq!(original, decode);
    }
}
