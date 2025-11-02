use super::trade::TimeStamp;
use chrono::Utc;

#[derive(Debug)]
pub struct TradeContext {
    pub trade_id: String,          // uuid7, use bech32
    pub witness_set: Vec<Witness>, // trade context
}

#[derive(Debug, PartialEq, Eq, minicbor::Encode, minicbor::Decode, Clone)]
pub struct Witness {
    #[n(0)]
    pub trade_id: String, // a unique string that is a reference to [`Trade`]
    #[n(1)]
    pub user_addr: String,
    #[n(2)]
    pub user_timestamp: TimeStamp<Utc>, // issued when the witness set is created
    #[n(3)]
    pub witness_type: WitnessType,
}

#[derive(Debug, PartialEq, Eq, minicbor::Encode, minicbor::Decode, Clone)]
pub enum WitnessType {
    #[n(0)]
    Submit {
        #[n(0)]
        details_hash: String, // hash of a trade-details object
        #[n(1)]
        requester_id: String,
        #[n(2)]
        approver_id: String,
    },
    #[n(1)]
    Approve,
    #[n(2)]
    Cancel,
    #[n(3)]
    Update {
        #[n(0)]
        details_hash: String,
    },
    #[n(4)]
    SendToExecute,
    #[n(5)]
    Book {
        #[n(0)]
        strike: u64,
    },
}

impl Witness {
    pub fn new(
        trade_id: String,
        user_addr: String,
        user_timestamp: TimeStamp<Utc>,
        witness_type: WitnessType,
    ) -> Self {
        Self {
            trade_id,
            user_addr,
            user_timestamp,
            witness_type,
        }
    }
    pub fn build(&self) -> anyhow::Result<(String, Vec<u8>)> {
        let cbor = minicbor::to_vec(self)?;
        let hash = sha256::digest(&cbor);

        Ok((hash, cbor))
    }
}

impl TradeContext {
    pub fn new(trade_id: String) -> Self {
        Self {
            trade_id,
            witness_set: vec![],
        }
    }
    pub fn insert_witness(&mut self, witness: Witness) {
        self.witness_set.push(witness);
    }
}

#[cfg(test)]
mod tests {
    use uuid7::uuid7;

    use super::*;
    use crate::{trade::*, utils};

    // demonstating adhoc way of going through the workflow
    #[test]
    fn build_trade() {
        let mut map = std::collections::HashMap::new();
        // create a new trade context to keep everything in order,
        let trade_id = utils::uudi_to_bech32("trade_").unwrap();
        let mut trade_context = TradeContext::new(trade_id.clone());

        let date_a = TimeStamp::new();
        let date_b = TimeStamp::new();
        let date_c = TimeStamp::new();

        // we fist construct the draft doc
        // on build we submit
        let trade_details = TradeDetails::new()
            .new_trade_entity(EntityID::new())
            .new_counter_party(EntityID::new())
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::EUR)
            .set_notional_amount(20_000)
            .set_underlying_amount(15_000)
            .set_underlying_currency(Currency::GBP)
            .set_trade_date(date_a)
            .set_value_date(date_b)
            .set_delivery_date(date_c);

        let draft_res = trade_details.build().unwrap();

        let witness_type = WitnessType::Submit {
            details_hash: draft_res.0.clone(),
            requester_id: uuid7().to_string(),
            approver_id: uuid7().to_string(),
        };

        map.insert(draft_res.0, draft_res.1);

        let witness = Witness::new(
            trade_id,
            utils::uudi_to_bech32("user_").unwrap(),
            TimeStamp::new(),
            witness_type,
        );

        // then insert into the map.
        let encode_witnes = witness.build().unwrap();
        map.insert(encode_witnes.0, encode_witnes.1);

        // insert out constructed witness set
        trade_context.insert_witness(witness);

        match &trade_context.witness_set[0].witness_type {
            WitnessType::Submit {
                details_hash,
                requester_id: _,
                approver_id: _,
            } => {
                if let Some(details) = map.get(details_hash) {
                    let trade: TradeDetails = minicbor::decode(details).unwrap();
                    println!("{:?}", trade);
                }
            }
            _ => {}
        }
    }
}
