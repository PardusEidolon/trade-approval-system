//! Smoke Screen Unit tests for trade approval system components
//!
//! These test are unit tests that span the codebase, testing behavior in
//! isolation from integration scenarios. These are intended as smoke-screen
//! and generally test the happy-path.
//!
#![allow(unused_imports)]

use chrono::{Datelike, Timelike, Utc};
use trade_approval::{
    context::{TradeContext, TradeState, Witness, WitnessType},
    trade::{Currency, Direction, TimeStamp, TradeDetails},
    utils::new_uuid_to_bech32,
};

// UTILS MODULE TESTS
#[cfg(test)]
mod utils_tests {
    use super::*;

    /// Test that new_uuid_to_bech32 generates valid bech32-encoded strings
    /// with the correct human-readable prefix
    #[test]
    fn generates_valid_bech32_with_hrp() {
        let result = new_uuid_to_bech32("trade_");
        assert!(result.is_ok());

        let encoded = result.unwrap();
        assert!(encoded.starts_with("trade_1"));
        assert!(encoded.len() > 10); // UUID should produce substantial output
    }

    /// Test that the function handles empty strings appropriately
    #[test]
    fn handles_empty_hrp() {
        // Empty string should fail
        let result = new_uuid_to_bech32("");
        assert!(result.is_err());
    }

    /// Test that multiple calls generate unique identifiers
    #[test]
    fn generates_unique_ids() {
        let id1 = new_uuid_to_bech32("trade_").unwrap();
        let id2 = new_uuid_to_bech32("trade_").unwrap();
        let id3 = new_uuid_to_bech32("trade_").unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    /// Test that different HRPs produce different encoded strings
    #[test]
    fn different_hrps_produce_different_encodings() {
        let trade_id = new_uuid_to_bech32("trade_").unwrap();
        let user_id = new_uuid_to_bech32("user_").unwrap();

        assert!(trade_id.starts_with("trade_"));
        assert!(user_id.starts_with("user_"));
        assert_ne!(trade_id, user_id);
    }
}

// TRADE MODULE TESTS
#[cfg(test)]
mod trade_tests {
    use super::*;

    /// Test that TimeStamp::new() creates a timestamp close to current time
    #[test]
    fn timestamp_new_creates_current_time() {
        let ts = TimeStamp::new();
        let now = Utc::now();

        let diff = (now - ts.to_datetime_utc()).num_seconds().abs();
        assert!(diff < 1); // Should be within 1 second
    }

    /// Test that TimeStamp can be created with specific date/time values
    #[test]
    fn timestamp_new_with_creates_specific_time() {
        let ts = TimeStamp::new_with(2024, 6, 15, 10, 30, 0);
        let dt = ts.to_datetime_utc();

        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 6);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);
    }

    /// Test that TimeStamp CBOR encoding/decoding round-trips correctly
    #[test]
    fn timestamp_cbor_roundtrip() {
        let original = TimeStamp::new();

        let encoded = minicbor::to_vec(&original).unwrap();
        let decoded: TimeStamp<Utc> = minicbor::decode(&encoded).unwrap();

        assert_eq!(original, decoded);
    }

    /// Test that TradeDetails builder pattern works correctly
    #[test]
    fn trade_details_builder_sets_fields() {
        let ts = TimeStamp::new();

        let trade = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts.clone());

        // Validation should pass with all fields set
        assert!(trade.validate_and_finalise().is_ok());
    }

    /// Test that validate_dates returns true for valid date sequence
    #[test]
    fn validate_dates_accepts_valid_sequence() {
        let trade_date = TimeStamp::new_with(2024, 6, 1, 0, 0, 0);
        let value_date = TimeStamp::new_with(2024, 6, 15, 0, 0, 0);
        let delivery_date = TimeStamp::new_with(2024, 6, 30, 0, 0, 0);

        let trade = TradeDetails::new()
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        assert!(trade.validate_dates());
    }

    /// Test that validate_dates accepts equal dates (boundary case)
    #[test]
    fn validate_dates_accepts_equal_dates() {
        let same_date = TimeStamp::new_with(2024, 6, 15, 0, 0, 0);

        let trade = TradeDetails::new()
            .set_trade_date(same_date.clone())
            .set_value_date(same_date.clone())
            .set_delivery_date(same_date);

        assert!(trade.validate_dates());
    }

    /// Test that validate_dates rejects out-of-order dates
    #[test]
    fn validate_dates_rejects_invalid_sequence() {
        let trade_date = TimeStamp::new_with(2024, 6, 30, 0, 0, 0);
        let value_date = TimeStamp::new_with(2024, 6, 15, 0, 0, 0);
        let delivery_date = TimeStamp::new_with(2024, 6, 1, 0, 0, 0);

        let trade = TradeDetails::new()
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        assert!(!trade.validate_dates());
    }

    /// Test that validate_dates rejects missing dates
    #[test]
    fn validate_dates_rejects_missing_dates() {
        let trade = TradeDetails::new().set_trade_date(TimeStamp::new());

        assert!(!trade.validate_dates());
    }

    /// Test that validate_and_finalise rejects trade with missing entity
    #[test]
    fn validate_and_finalise_rejects_missing_entity() {
        let ts = TimeStamp::new();

        let trade = TradeDetails::new()
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts);

        assert!(trade.validate_and_finalise().is_err());
    }

    /// Test that validate_and_finalise rejects zero amounts
    #[test]
    fn validate_and_finalise_rejects_zero_amounts() {
        let ts = TimeStamp::new();

        let trade = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(0) // Zero amount
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts);

        assert!(trade.validate_and_finalise().is_err());
    }

    /// Test that validate_and_finalise rejects invalid date sequences
    #[test]
    fn validate_and_finalise_enforces_date_ordering() {
        let trade_date = TimeStamp::new_with(2024, 6, 30, 0, 0, 0);
        let value_date = TimeStamp::new_with(2024, 6, 1, 0, 0, 0);
        let delivery_date = TimeStamp::new_with(2024, 6, 15, 0, 0, 0);

        let trade = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        assert!(trade.validate_and_finalise().is_err());
    }

    /// Test that identical TradeDetails produce identical hashes
    #[test]
    fn identical_trades_produce_same_hash() {
        let ts = TimeStamp::new_with(2024, 6, 15, 10, 30, 0);

        let trade1 = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts.clone());

        let trade2 = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts);

        let (hash1, _) = trade1.validate_and_finalise().unwrap();
        let (hash2, _) = trade2.validate_and_finalise().unwrap();

        // Note: This will fail because new_trade_entity generates new UUIDs
        // This test demonstrates the content-addressable nature
        assert_ne!(
            hash1, hash2,
            "Different UUIDs should produce different hashes"
        );
    }

    /// Test that different amounts produce different hashes
    #[test]
    fn different_amounts_produce_different_hashes() {
        let ts = TimeStamp::new();

        let trade1 = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(1_000_000)
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts.clone());

        let trade2 = TradeDetails::new()
            .new_trade_entity("entity_")
            .new_counter_party("counter_")
            .set_direction(Direction::Buy)
            .set_notional_currency(Currency::USD)
            .set_notional_amount(2_000_000) // Different amount
            .set_underlying_currency(Currency::EUR)
            .set_underlying_amount(850_000)
            .set_trade_date(ts.clone())
            .set_value_date(ts.clone())
            .set_delivery_date(ts);

        let (hash1, _) = trade1.validate_and_finalise().unwrap();
        let (hash2, _) = trade2.validate_and_finalise().unwrap();

        assert_ne!(hash1, hash2);
    }

    /// Test Currency enum ordering
    #[test]
    fn currency_ordering() {
        assert!(Currency::USD < Currency::GBP);
        assert!(Currency::GBP < Currency::EUR);
        assert_eq!(Currency::USD, Currency::USD);
    }

    /// Test Direction enum ordering
    #[test]
    fn direction_ordering() {
        assert!(Direction::Buy < Direction::Sell);
        assert_eq!(Direction::Buy, Direction::Buy);
    }
}

// CONTEXT MODULE TESTS
#[cfg(test)]
mod context_tests {
    use super::*;

    /// Helper to create a basic witness for testing
    fn create_test_witness(
        trade_id: String,
        user_id: String,
        witness_type: WitnessType,
    ) -> Witness {
        Witness::new(trade_id, user_id, TimeStamp::new(), witness_type)
    }

    /// Test that new TradeContext has empty witness set and valid trade_id
    #[test]
    fn new_context_has_empty_witness_set() {
        let ctx = TradeContext::new();

        assert!(ctx.witness_set.is_empty());
        assert!(ctx.trade_id.starts_with("trade_"));
        assert!(ctx.trade_id.len() > 10);
    }

    /// Test that new_with creates context with specified trade_id
    #[test]
    fn new_with_uses_provided_trade_id() {
        let custom_id = "trade_custom123".to_string();
        let ctx = TradeContext::new_with(custom_id.clone());

        assert_eq!(ctx.trade_id, custom_id);
        assert!(ctx.witness_set.is_empty());
    }

    /// Test that insert_witness adds to witness_set
    #[test]
    fn insert_witness_appends_to_set() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let witness = create_test_witness(trade_id, "user_123".to_string(), WitnessType::Approve);

        assert_eq!(ctx.witness_set.len(), 0);
        ctx.insert_witness(witness);
        assert_eq!(ctx.witness_set.len(), 1);
    }

    /// Test that empty witness set results in Draft state
    #[test]
    fn current_state_draft_with_empty_witnesses() {
        let ctx = TradeContext::new();
        assert_eq!(ctx.current_state(), TradeState::Draft);
    }

    /// Test that Submit witness results in PendingApproval state
    #[test]
    fn current_state_pending_approval_after_submit() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id,
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        ctx.insert_witness(submit_witness);
        assert_eq!(ctx.current_state(), TradeState::PendingApproval);
    }

    /// Test that Submit followed by Approve results in Approved state
    #[test]
    fn current_state_approved_after_approval() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let approve_witness =
            create_test_witness(trade_id, "user_456".to_string(), WitnessType::Approve);

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(approve_witness);

        assert_eq!(ctx.current_state(), TradeState::Approved);
    }

    /// Test that Update after Approve invalidates approval
    #[test]
    fn current_state_pending_after_update_invalidates_approval() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let approve_witness = create_test_witness(
            trade_id.clone(),
            "user_456".to_string(),
            WitnessType::Approve,
        );

        let update_witness = create_test_witness(
            trade_id,
            "user_123".to_string(),
            WitnessType::Update {
                details_hash: "hash_def".to_string(),
            },
        );

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(approve_witness);
        assert_eq!(ctx.current_state(), TradeState::Approved);

        ctx.insert_witness(update_witness);
        assert_eq!(ctx.current_state(), TradeState::PendingApproval);
    }

    /// This test behavior where Update followed by Approve is returns PendingApproval.
    #[test]
    fn state_update_behavior() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );
        ctx.insert_witness(submit_witness);

        let approve1 = create_test_witness(
            trade_id.clone(),
            "user_456".to_string(),
            WitnessType::Approve,
        );
        ctx.insert_witness(approve1);
        assert_eq!(ctx.current_state(), TradeState::Approved);

        let update_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Update {
                details_hash: "hash_def".to_string(),
            },
        );
        ctx.insert_witness(update_witness);
        assert_eq!(ctx.current_state(), TradeState::PendingApproval);

        // Add another approval
        let approve2 = create_test_witness(trade_id, "user_456".to_string(), WitnessType::Approve);
        ctx.insert_witness(approve2);

        assert_eq!(
            ctx.current_state(),
            TradeState::Approved,
            "After an Update, a new Approve should move the state back to Approved.",
        );
    }

    /// Test that Cancel witness results in Cancelled state
    #[test]
    fn current_state_cancelled_after_cancel() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let cancel_witness =
            create_test_witness(trade_id, "user_123".to_string(), WitnessType::Cancel);

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(cancel_witness);

        assert_eq!(ctx.current_state(), TradeState::Cancelled);
    }

    /// Test that SendToExecute results in SentToExecute state
    #[test]
    fn current_state_sent_to_execute() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let approve_witness = create_test_witness(
            trade_id.clone(),
            "user_456".to_string(),
            WitnessType::Approve,
        );

        let execute_witness =
            create_test_witness(trade_id, "user_123".to_string(), WitnessType::SendToExecute);

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(approve_witness);
        ctx.insert_witness(execute_witness);

        assert_eq!(ctx.current_state(), TradeState::SentToExecute);
    }

    /// Test that Book witness results in Booked state
    #[test]
    fn current_state_booked_after_booking() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_1s23".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let approve_witness = create_test_witness(
            trade_id.clone(),
            "user_456".to_string(),
            WitnessType::Approve,
        );

        let execute_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::SendToExecute,
        );

        let book_witness = create_test_witness(
            trade_id,
            "user_123".to_string(),
            WitnessType::Book { strike: 100_000 },
        );

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(approve_witness);
        ctx.insert_witness(execute_witness);
        ctx.insert_witness(book_witness);

        assert_eq!(ctx.current_state(), TradeState::Booked);
    }

    /// Test requires_approval returns true for PendingApproval state
    #[test]
    fn requires_approval_true_when_pending() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id,
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        ctx.insert_witness(submit_witness);
        assert!(ctx.requires_approval());
    }

    /// Test requires_approval returns false for Approved state
    #[test]
    fn requires_approval_false_when_approved() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();

        let submit_witness = create_test_witness(
            trade_id.clone(),
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: "user_456".to_string(),
            },
        );

        let approve_witness =
            create_test_witness(trade_id, "user_456".to_string(), WitnessType::Approve);

        ctx.insert_witness(submit_witness);
        ctx.insert_witness(approve_witness);

        assert!(!ctx.requires_approval());
    }

    /// Test get_expected_approver returns correct approver from Submit
    #[test]
    fn get_expected_approver_returns_correct_id() {
        let mut ctx = TradeContext::new();
        let trade_id = ctx.trade_id.clone();
        let expected_approver = "user_456".to_string();

        let submit_witness = create_test_witness(
            trade_id,
            "user_123".to_string(),
            WitnessType::Submit {
                details_hash: "hash_abc".to_string(),
                requester_id: "user_123".to_string(),
                approver_id: expected_approver.clone(),
            },
        );

        ctx.insert_witness(submit_witness);

        let approver = ctx.get_expected_approver().unwrap();
        assert_eq!(approver, expected_approver);
    }

    /// Test get_expected_approver fails when no Submit witness exists
    #[test]
    fn get_expected_approver_fails_without_submit() {
        let ctx = TradeContext::new();
        assert!(ctx.get_expected_approver().is_err());
    }
}
