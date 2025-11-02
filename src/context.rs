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

    // demonstating adhoc way of going through the transitions
    #[test]
    fn adhoc_trade_workflow() {
        let mut map = std::collections::HashMap::new();

        // create a new trade context. this contains the trade name and witnesses.
        let trade_id = utils::uudi_to_bech32("trade_").unwrap();
        let mut trade_context = TradeContext::new(trade_id.clone());

        let date_a = TimeStamp::new();
        let date_b = TimeStamp::new();
        let date_c = TimeStamp::new();

        // we fist construct the draft doc. Then on build we submit
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

        // on build we go through a series of validation checks then on success we return a serialsed format of the trade and it's hash.
        let draft_res = trade_details.build().unwrap();

        // we need to contruct a witnesstype in our case the action or (state transition) which keeps a copy of the hash for future lookups
        let witness_type = WitnessType::Submit {
            details_hash: draft_res.0.clone(),
            requester_id: uuid7().to_string(),
            approver_id: uuid7().to_string(),
        };
        // then insert the encoded trade with its hash into the map.
        map.insert(draft_res.0, draft_res.1);

        // the witness type is then used to contain the nested witness type of our action then we store an id of the trade as this is important to being able to trace back.
        let witness = Witness::new(
            trade_id,
            utils::uudi_to_bech32("user_").unwrap(),
            TimeStamp::new(),
            witness_type,
        );

        // it's the same behvaiour as the builder for the trade details. return then encoded data and it's hash then insert into our map.
        let encode_witnes = witness.build().unwrap();
        map.insert(encode_witnes.0, encode_witnes.1);
        // we also want to keep a copy in our trading context
        trade_context.insert_witness(witness);

        // next is to retrieve and perform equality checks
        match &trade_context.witness_set[0].witness_type {
            WitnessType::Submit {
                details_hash,
                requester_id: _,
                approver_id: _,
            } => {
                if let Some(details) = map.get(details_hash) {
                    let trade: TradeDetails = minicbor::decode(details).unwrap();
                    assert_eq!(trade_details, trade)
                }
            }
            _ => {}
        }
        // assert equals on the witness from the one stored in our hashmap
        match &trade_context.witness_set[0] {
            wtns => {
                // need to derive a hash
                let encoded = minicbor::to_vec(wtns).unwrap();
                let hash = sha256::digest(&encoded);

                if let Some(stored_witness) = map.get(&hash) {
                    let data: Witness = minicbor::decode(&stored_witness).unwrap();
                    assert_eq!(*wtns, data)
                }
            }
        }
    }
}
