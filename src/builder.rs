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

// used for constructing drafts
#[derive(Default)]
pub struct TradeDetailsBuilder {
    // No ID field, as the ID *is* the hash of this struct
    pub trading_entity: Option<UserID>,
    pub counter_party: Option<UserID>,
    pub direction: Option<Direction>,
    pub notional_currency: Option<Currency>,
    pub notional_amount: u64,
    pub underlying_currency: Option<Currency>,
    pub underlying_amount: u64,
    pub trade_date: Option<TimeStamp<Utc>>,
    pub value_date: Option<TimeStamp<Utc>>,
    pub delivery_date: Option<TimeStamp<Utc>>,
    pub strike: Option<u64>,
}

// key is the hash of this struct encoded into cbor
#[derive(minicbor::Encode, minicbor::Decode, Debug)]
pub struct TradeDetails {
    // No ID field, as the ID *is* the hash of this struct
    #[n(0)]
    pub trading_entity: EntityID, // Wallet Address
    #[n(1)]
    pub counter_party: EntityID, // Wallet Address
    #[n(2)]
    pub direction: Direction,
    #[n(3)]
    pub notional_currency: Currency,
    #[n(4)]
    pub notional_amount: u64,
    #[n(5)]
    pub underlying_currency: Currency,
    #[n(6)]
    pub underlying_amount: u64,
    #[n(7)]
    pub trade_date: TimeStamp<Utc>,
    #[n(8)]
    pub value_date: TimeStamp<Utc>,
    #[n(9)]
    pub delivery_date: TimeStamp<Utc>,
    #[n(10)]
    pub strike: u64, // The agreed upon rate
}

// newtype wrapper over uuid because Uuid doesn't implement minicbor traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserID(Uuid);

#[derive(Debug, minicbor::Decode, minicbor::Encode)]
#[cbor(array)]
pub struct EntityID(#[n(0)] UserID); // uuid7 type strin

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
}

impl TradeDetailsBuilder {
    /// Construct a new builder object, this becomes the basis for a draft
    pub fn new() -> Self {
        Self::default()
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
