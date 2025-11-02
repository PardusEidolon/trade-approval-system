use super::trade::TimeStamp;
use chrono::Utc;

#[derive(Debug, PartialEq, Eq)]
pub struct Witness {
    pub trade_id: String, // a hash which references the current trade in our db
    pub user_addr: String,
    pub timestamp_utc: TimeStamp<Utc>,
    pub witness_type: WitnessType,
}

#[derive(Debug, PartialEq, Eq)]
pub enum WitnessType {
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
