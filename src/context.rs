#![allow(dead_code)]
//! Trade context and witness management for state derivation

use super::trade::TimeStamp;
use super::utils::new_uuid_to_bech32;
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeState {
    Draft,           // No Submit yet
    PendingApproval, // Latest action is Submit or Update
    Approved,        // Latest action is Approve (and no Update after)
    Cancelled,
    SentToExecute,
    Executed,
    Booked,
}

#[derive(Debug, minicbor::Encode, minicbor::Decode)]
pub struct TradeContext {
    /// uses a bech2_encoded uuid string. this string is also referenced in the witness
    #[n(0)]
    pub trade_id: String,
    #[n(1)]
    pub witness_set: Vec<Witness>,
}

#[derive(Debug, PartialEq, Eq, minicbor::Encode, minicbor::Decode, Clone)]
pub struct Witness {
    #[n(0)]
    pub trade_id: String,
    /// a unique string that is a reference to the trade
    #[n(1)]
    pub user_id: String,
    #[n(2)]
    pub user_timestamp: TimeStamp<Utc>,
    /// issued when the witness set is created
    #[n(3)]
    pub witness_type: WitnessType,
}

#[derive(Debug, PartialEq, Eq, minicbor::Encode, minicbor::Decode, Clone)]
pub enum WitnessType {
    /// if we pass validaion checks on build we are in the pending approval stage
    #[n(0)]
    Submit {
        #[n(0)]
        details_hash: String, // hash of a trade-details object
        #[n(1)]
        requester_id: String,
        #[n(2)]
        approver_id: String, // who is responsible to approving the trade
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

/// primary action type that drives the trade.
impl WitnessType {
    fn new_submit(details_hash: String, requester_id: String, approver_id: String) -> Self {
        Self::Submit {
            details_hash,
            requester_id,
            approver_id,
        }
    }
    fn new_update(details_hash: String) -> Self {
        Self::Update { details_hash }
    }
    fn new_book(strike: u64) -> Self {
        Self::Book { strike }
    }
}

impl Witness {
    pub fn new(
        trade_id: String,
        user_id: String,
        user_timestamp: TimeStamp<Utc>,
        witness_type: WitnessType,
    ) -> Self {
        Self {
            trade_id,
            user_id,
            user_timestamp,
            witness_type,
        }
    }
    /// encode to cbor then return the hassh and the encoded contents.
    pub fn serialize_with_hash(&self) -> anyhow::Result<(String, Vec<u8>)> {
        let cbor = minicbor::to_vec(self)?;
        let id = self.trade_id.clone();

        Ok((id, cbor))
    }
}
impl TradeContext {
    pub fn new() -> Self {
        let trade_id = new_uuid_to_bech32("trade_").expect("generate new ID for trade_context ");
        Self {
            trade_id,
            witness_set: vec![],
        }
    }
    // generate a uuid outside this types context
    pub fn new_with(trade_id: String) -> Self {
        Self {
            trade_id,
            witness_set: vec![],
        }
    }
    pub fn insert_witness(&mut self, witness: Witness) {
        self.witness_set.push(witness);
    }

    /// Serialize to CBOR with content hash for integrity
    pub fn serialize_with_hash(&self) -> anyhow::Result<(String, Vec<u8>)> {
        let cbor = minicbor::to_vec(self)?;
        let hash = sha256::digest(&cbor);
        Ok((hash, cbor))
    }

    /// Save to database using trade_id as key
    pub fn save_to_db(&self, db: &sled::Db) -> anyhow::Result<String> {
        let (content_hash, cbor) = self.serialize_with_hash()?;

        // Use trade_id (unhashed) as the key
        db.insert(self.trade_id.as_bytes(), cbor)?;

        // Return hash for audit/verification purposes
        Ok(content_hash)
    }

    /// Load from database using trade_id
    pub fn load_from_db(db: &sled::Db, trade_id: &str) -> anyhow::Result<Self> {
        let bytes = db
            .get(trade_id.as_bytes())?
            .ok_or_else(|| anyhow::anyhow!("Trade not found: {}", trade_id))?;

        let trade_context: TradeContext = minicbor::decode(&bytes)?;
        Ok(trade_context)
    }

    /// Determine current state by examining witness chain
    pub fn current_state(&self) -> TradeState {
        if self.witness_set.is_empty() {
            return TradeState::Draft;
        }

        // Walk backwards to find the latest relevant state
        let mut approved = false;

        for witness in self.witness_set.iter().rev() {
            match &witness.witness_type {
                WitnessType::Submit { .. } => {
                    return TradeState::PendingApproval;
                }
                WitnessType::Update { .. } => {
                    // Update invalidates previous approval
                    return TradeState::PendingApproval;
                }
                WitnessType::Approve => {
                    approved = true;
                    // Keep checking - might be an Update after this
                }
                WitnessType::Cancel => {
                    return TradeState::Cancelled;
                }
                WitnessType::SendToExecute => {
                    return TradeState::SentToExecute;
                }
                WitnessType::Book { .. } => {
                    return TradeState::Booked;
                }
            }
        }

        // If we get here and saw Approve, and no Submit/Update after
        if approved {
            TradeState::Approved
        } else {
            TradeState::Draft
        }
    }

    /// Check if trade needs approval before proceeding
    pub fn requires_approval(&self) -> bool {
        matches!(self.current_state(), TradeState::PendingApproval)
    }

    /// Get the expected approver from the latest Submit or Update with approver info
    pub fn get_expected_approver(&self) -> anyhow::Result<String> {
        // Walk backwards to find the latest Submit or Update
        for witness in self.witness_set.iter().rev() {
            match &witness.witness_type {
                WitnessType::Submit { approver_id, .. } => {
                    return Ok(approver_id.clone());
                }
                WitnessType::Update { .. } => {
                    // Update doesn't have approver_id, need to find previous Submit
                    continue;
                }
                _ => continue,
            }
        }

        Err(anyhow::anyhow!(
            "No Submit witness found with approver information"
        ))
    }
}
