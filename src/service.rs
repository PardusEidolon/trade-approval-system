//! Serves as the primary entry-point for conducting trades

use super::context::{self, TradeContext};

pub struct TradeService {
    instance: sled::Db,
    context: TradeContext,
}

// TODO: implement the service entry-point tmrw

impl TradeService {}
