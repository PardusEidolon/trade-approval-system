//! Service layer API for trade workflow operations
use super::context::{TradeContext, TradeState, Witness, WitnessType};
use super::trade::{TimeStamp, TradeDetails};
use chrono::Utc;
use sled::{Batch, Db};
use std::sync::Arc;

pub struct TradeService {
    instance: Arc<sled::Db>,
    // in future we could add a config for approval contraints
}

impl TradeService {
    pub fn new(instance: Arc<sled::Db>) -> Self {
        Self { instance }
    }

    /// Load trade context from database
    fn load_trade_context(&self, trade_id: &str) -> anyhow::Result<TradeContext> {
        TradeContext::load_from_db(&self.instance, trade_id)
    }

    /// Submit a new trade for approval
    pub fn submit_trade(
        &self,
        trade_details: TradeDetails,
        requester_id: String,
        approver_id: String,
        user_addr: String,
    ) -> anyhow::Result<TradeContext> {
        // Validate and serialize trade details
        let (details_hash, details_cbor) = trade_details.validate_and_finalise()?;

        // Create new trade context
        let mut trade_context = TradeContext::new();

        // Create Submit witness
        let witness = Witness::new(
            trade_context.trade_id.clone(),
            user_addr,
            TimeStamp::new(),
            WitnessType::Submit {
                details_hash: details_hash.clone(),
                requester_id,
                approver_id,
            },
        );

        // Add witness to context
        trade_context.insert_witness(witness);

        // Batch insert: trade details and trade context
        let mut batch = Batch::default();
        // insert trade details
        batch.insert(details_hash.as_bytes(), details_cbor);
        // insert context with witness
        batch.insert(
            trade_context.trade_id.as_bytes(),
            minicbor::to_vec(&trade_context)?,
        );
        self.instance.apply_batch(batch)?;

        Ok(trade_context)
    }

    /// Approve a trade that is in PendingApproval state
    pub fn approve_trade(
        &self,
        trade_id: String,
        approver_id: String,
    ) -> anyhow::Result<TradeContext> {
        // Load from DB
        let mut trade_context = self.load_trade_context(&trade_id)?;

        // Verify it's in a state that needs approval
        if !trade_context.requires_approval() {
            return Err(anyhow::anyhow!(
                "Trade does not require approval. Current state: {:?}",
                trade_context.current_state()
            ));
        }

        // Verify approver_id matches the one in latest Submit
        let expected_approver = trade_context.get_expected_approver()?;
        if approver_id != expected_approver {
            return Err(anyhow::anyhow!(
                "Unauthorized approver. Expected: {}, Got: {}",
                expected_approver,
                approver_id
            ));
        }

        // Add Approve witness
        let witness = Witness::new(
            trade_id.clone(),
            approver_id,
            TimeStamp::new(),
            WitnessType::Approve,
        );
        trade_context.insert_witness(witness);

        // Save back to DB
        trade_context.save_to_db(&self.instance)?;

        Ok(trade_context)
    }

    /// Update trade details (requires re-approval)
    pub fn update_trade(
        &self,
        trade_id: String,
        trade_details: TradeDetails,
        user_addr: String,
    ) -> anyhow::Result<TradeContext> {
        // Load existing trade context
        let mut trade_context = self.load_trade_context(&trade_id)?;

        // Validate and serialize new trade details
        let (details_hash, details_cbor) = trade_details.validate_and_finalise()?;

        // Create Update witness
        let witness = Witness::new(
            trade_id.clone(),
            user_addr,
            TimeStamp::new(),
            WitnessType::Update {
                details_hash: details_hash.clone(),
            },
        );

        // Add witness to context
        trade_context.insert_witness(witness);

        // Batch insert: new trade details and updated trade context
        let mut batch = Batch::default();
        batch.insert(details_hash.as_bytes(), details_cbor);
        batch.insert(
            trade_context.trade_id.as_bytes(),
            minicbor::to_vec(&trade_context)?,
        );
        self.instance.apply_batch(batch)?;

        Ok(trade_context)
    }

    /// Cancel a trade
    pub fn cancel_trade(
        &self,
        trade_id: String,
        user_addr: String,
    ) -> anyhow::Result<TradeContext> {
        // Load existing trade context
        let mut trade_context = self.load_trade_context(&trade_id)?;

        // Create Cancel witness
        let witness = Witness::new(
            trade_id.clone(),
            user_addr,
            TimeStamp::new(),
            WitnessType::Cancel,
        );

        // Add witness to context
        trade_context.insert_witness(witness);

        // Save to DB
        trade_context.save_to_db(&self.instance)?;

        Ok(trade_context)
    }

    /// Send approved trade to execution
    pub fn execute_trade(
        &self,
        trade_id: String,
        user_addr: String,
    ) -> anyhow::Result<TradeContext> {
        // Load existing trade context
        let mut trade_context = self.load_trade_context(&trade_id)?;

        // Verify trade is approved before execution
        if trade_context.current_state() != TradeState::Approved {
            return Err(anyhow::anyhow!(
                "Trade must be approved before execution. Current state: {:?}",
                trade_context.current_state()
            ));
        }

        // Create SendToExecute witness
        let witness = Witness::new(
            trade_id.clone(),
            user_addr,
            TimeStamp::new(),
            WitnessType::SendToExecute,
        );

        // Add witness to context
        trade_context.insert_witness(witness);

        // Save to DB
        trade_context.save_to_db(&self.instance)?;

        Ok(trade_context)
    }

    /// Book an executed trade
    pub fn book_trade(
        &self,
        trade_id: String,
        user_addr: String,
        strike: u64,
    ) -> anyhow::Result<TradeContext> {
        // Load existing trade context
        let mut trade_context = self.load_trade_context(&trade_id)?;

        // Create Book witness
        let witness = Witness::new(
            trade_id.clone(),
            user_addr,
            TimeStamp::new(),
            WitnessType::Book { strike },
        );

        // Add witness to context
        trade_context.insert_witness(witness);

        // Save to DB
        trade_context.save_to_db(&self.instance)?;

        Ok(trade_context)
    }
}
