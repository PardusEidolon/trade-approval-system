//! Property-based tests for witness chain state derivation
//!
//! This module uses proptest to verify that the state machine logic in TradeContext
//! behaves correctly across a wide variety of witness sequences. The state derivation
//! logic is critical - bugs here corrupt the entire trade workflow.
//!
//! These tests focus on invariants that should hold regardless of the specific witness
//! sequence, helping catch edge cases in the state machine that would be difficult to
//! find with manual test case selection.

use proptest::prelude::*;
use trade_approval::{
    context::{TradeContext, TradeState, Witness, WitnessType},
    trade::TimeStamp,
};

// These property tests cover:
//
// 1. Idempotency - fundamental correctness requirement
// 2. Terminal state stability - ensures workflow endpoints are truly final
// 3. Base case (empty context) - validates initial conditions
// 4. Consistency between related methods - prevents API confusion
// 5. Serialization correctness - critical for persistence
// 6. Basic approval workflow - validates happy path
// 7. Update invalidation - validates critical business rule
//
// What these tests DON'T cover (deliberately):
//
// - Database persistence (requires mocking/tempfile, better in integration tests)
// - Authorization checks (handled by service layer, not state derivation)
//

/// Strategy to generate a valid witness type
fn witness_type_strategy() -> impl Strategy<Value = WitnessType> {
    prop_oneof![
        // Submit with generated IDs
        (any::<u32>(), any::<u32>(), any::<u32>()).prop_map(|(h, r, a)| {
            WitnessType::Submit {
                details_hash: format!("hash_{}", h),
                requester_id: format!("user_{}", r),
                approver_id: format!("user_{}", a),
            }
        }),
        Just(WitnessType::Approve),
        Just(WitnessType::Cancel),
        any::<u32>().prop_map(|h| WitnessType::Update {
            details_hash: format!("hash_{}", h),
        }),
        Just(WitnessType::SendToExecute),
        any::<u64>().prop_map(|strike| WitnessType::Book { strike }),
    ]
}

/// Strategy to generate a witness with a given trade_id
fn witness_strategy(trade_id: String) -> impl Strategy<Value = Witness> {
    (any::<u32>(), witness_type_strategy()).prop_map(move |(user_num, witness_type)| {
        Witness::new(
            trade_id.clone(),
            format!("user_{}", user_num),
            TimeStamp::new(),
            witness_type,
        )
    })
}

/// Strategy to generate a sequence of witnesses (1 to 10 witnesses)
fn witness_sequence_strategy(trade_id: String) -> impl Strategy<Value = Vec<Witness>> {
    prop::collection::vec(witness_strategy(trade_id), 1..=10)
}

/// Strategy to generate a witness sequence that starts with Submit
/// (required for valid trade workflows)
fn valid_workflow_strategy(trade_id: String) -> impl Strategy<Value = Vec<Witness>> {
    (any::<u32>(), any::<u32>(), any::<u32>()).prop_flat_map(move |(h, r, a)| {
        let trade_id = trade_id.clone();
        let submit = Witness::new(
            trade_id.clone(),
            format!("user_{}", r),
            TimeStamp::new(),
            WitnessType::Submit {
                details_hash: format!("hash_{}", h),
                requester_id: format!("user_{}", r),
                approver_id: format!("user_{}", a),
            },
        );

        // Generate 0-9 additional witnesses after Submit
        prop::collection::vec(witness_strategy(trade_id), 0..=9).prop_map(move |mut rest| {
            let mut sequence = vec![submit.clone()];
            sequence.append(&mut rest);
            sequence
        })
    })
}

// PROPERTY TESTS
proptest! {
    /// Property: current_state() is idempotent - calling it multiple times returns the same result
    ///
    /// This is fundamental: state derivation must be deterministic and have no side effects.
    /// If this fails, the state machine logic is fundamentally broken.
    #[test]
    fn prop_current_state_is_idempotent(
        witnesses in witness_sequence_strategy("trade_test123".to_string())
    ) {
        let mut ctx = TradeContext::new_with("trade_test123".to_string());

        for witness in witnesses {
            ctx.insert_witness(witness);
        }

        // Call current_state multiple times - should always return the same value
        let state1 = ctx.current_state();
        let state2 = ctx.current_state();
        let state3 = ctx.current_state();

        prop_assert_eq!(&state1, &state2, "First and second state should match");
        prop_assert_eq!(&state2, &state3, "Second and third state should match");
    }

    /// Property: Terminal states are equally terminal and stable
    ///
    /// Both Book and Cancel are terminal states. Once a trade reaches either state,
    /// it cannot transition to any other state, including the other terminal state.
    ///
    /// Business rules:
    /// - Once booked, cannot be cancelled (Book is permanent)
    /// - Once cancelled, cannot be booked (Cancel is permanent)
    /// - First terminal witness in the chain determines the final state
    #[test]
    fn prop_terminal_states_are_stable(
        initial_witnesses in valid_workflow_strategy("trade_test456".to_string()),
        terminal_type in prop_oneof![
            any::<u64>().prop_map(|strike| WitnessType::Book { strike }),
            Just(WitnessType::Cancel),
        ],
        additional_witnesses in prop::collection::vec(
            witness_strategy("trade_test456".to_string()),
            0..=5
        ),
    ) {
        let mut ctx = TradeContext::new_with("trade_test456".to_string());

        // Add initial witnesses
        for witness in initial_witnesses {
            ctx.insert_witness(witness);
        }

        // Add terminal witness
        let terminal_witness = Witness::new(
            "trade_test456".to_string(),
            "user_terminal".to_string(),
            TimeStamp::new(),
            terminal_type.clone(),
        );
        ctx.insert_witness(terminal_witness);

        let terminal_state = ctx.current_state();
        prop_assert!(
            matches!(terminal_state, TradeState::Booked | TradeState::Cancelled),
            "Should be in terminal state after adding terminal witness"
        );

        // Add more witnesses
        for witness in additional_witnesses.iter() {
            ctx.insert_witness(witness.clone());
        }

        let final_state = ctx.current_state();

        // Terminal states are equally terminal - first one wins
        // The state after adding more witnesses MUST remain the same
        // because terminal states cannot be overridden
        prop_assert_eq!(
            &terminal_state,
            &final_state,
            "Terminal state must remain stable - once booked, cannot be cancelled; once cancelled, cannot be booked"
        );
    }

    /// Property: Empty witness set always results in Draft state
    ///
    /// This is the base case for state derivation. A trade with no actions
    /// is always in Draft state.
    #[test]
    fn prop_empty_context_is_draft(trade_id in "[a-z]{5,10}") {
        let ctx = TradeContext::new_with(trade_id);
        prop_assert_eq!(
            &ctx.current_state(),
            &TradeState::Draft,
            "Empty context should always be Draft"
        );
    }

    /// Property: requires_approval() is consistent with current_state()
    ///
    /// requires_approval() should return true if and only if current_state()
    /// returns PendingApproval. These two methods must stay in sync.
    #[test]
    fn prop_requires_approval_consistent_with_state(
        witnesses in witness_sequence_strategy("trade_test789".to_string())
    ) {
        let mut ctx = TradeContext::new_with("trade_test789".to_string());

        for witness in witnesses {
            ctx.insert_witness(witness);
        }

        let state = ctx.current_state();
        let requires_approval = ctx.requires_approval();

        if requires_approval {
            prop_assert_eq!(
                &state,
                &TradeState::PendingApproval,
                "If requires_approval is true, state must be PendingApproval"
            );
        } else {
            prop_assert_ne!(
                &state,
                &TradeState::PendingApproval,
                "If requires_approval is false, state must not be PendingApproval"
            );
        }
    }

    /// Property: CBOR serialization round-trip preserves witness chain
    ///
    /// Critical for persistence: encoding then decoding a TradeContext must
    /// produce an identical witness chain and derive the same state.
    #[test]
    fn prop_cbor_roundtrip_preserves_state(
        witnesses in witness_sequence_strategy("trade_test999".to_string())
    ) {
        let mut original_ctx = TradeContext::new_with("trade_test999".to_string());

        for witness in witnesses {
            original_ctx.insert_witness(witness);
        }

        let original_state = original_ctx.current_state();
        let original_witness_count = original_ctx.witness_set.len();

        // Serialize and deserialize
        let (_hash, cbor) = original_ctx.serialize_with_hash()
            .expect("Serialization should succeed");

        let decoded_ctx: TradeContext = minicbor::decode(&cbor)
            .expect("Deserialization should succeed");

        let decoded_state = decoded_ctx.current_state();
        let decoded_witness_count = decoded_ctx.witness_set.len();

        prop_assert_eq!(
            original_witness_count,
            decoded_witness_count,
            "Witness count should be preserved"
        );

        prop_assert_eq!(
            &original_state,
            &decoded_state,
            "State should be preserved after round-trip"
        );
    }
}

// TARGETED PROPERTY TESTS FOR SPECIFIC INVARIANTS

proptest! {
    /// Property: Submit followed immediately by Approve results in Approved state
    ///
    /// This tests the most basic approval workflow. The property verifies that
    /// the happy path works correctly regardless of the specific IDs used.
    #[test]
    fn prop_submit_then_approve_is_approved(
        hash_num in any::<u32>(),
        requester_num in any::<u32>(),
        approver_num in any::<u32>(),
    ) {
        let mut ctx = TradeContext::new_with("trade_approval_test".to_string());

        let submit = Witness::new(
            "trade_approval_test".to_string(),
            format!("user_{}", requester_num),
            TimeStamp::new(),
            WitnessType::Submit {
                details_hash: format!("hash_{}", hash_num),
                requester_id: format!("user_{}", requester_num),
                approver_id: format!("user_{}", approver_num),
            },
        );

        let approve = Witness::new(
            "trade_approval_test".to_string(),
            format!("user_{}", approver_num),
            TimeStamp::new(),
            WitnessType::Approve,
        );

        ctx.insert_witness(submit);
        prop_assert_eq!(
            &ctx.current_state(),
            &TradeState::PendingApproval,
            "After Submit, state should be PendingApproval"
        );

        ctx.insert_witness(approve);
        prop_assert_eq!(
            &ctx.current_state(),
            &TradeState::Approved,
            "After Approve, state should be Approved"
        );
    }

    /// Property: Update after Approve always results in PendingApproval
    ///
    /// This is a critical business rule: any update to trade details invalidates
    /// the previous approval, requiring re-approval. This should hold regardless
    /// of the specific witness sequence, UNLESS the trade has already been booked
    /// (since Book is terminal and overrides everything).
    #[test]
    fn prop_update_invalidates_approval(
        initial_witnesses in valid_workflow_strategy("trade_update_test".to_string()),
        update_hash in any::<u32>(),
    ) {
        let mut ctx = TradeContext::new_with("trade_update_test".to_string());

        // Add initial witnesses (starts with Submit)
        for witness in initial_witnesses {
            ctx.insert_witness(witness);
        }

        // Skip this test if there's already a Book witness (terminal state)
        // or a Cancel witness (also terminal)
        let has_terminal = ctx.witness_set.iter().any(|w| {
            matches!(w.witness_type, WitnessType::Book { .. } | WitnessType::Cancel)
        });

        if has_terminal {
            // Can't test update invalidation on terminal states
            return Ok(());
        }

        // Add an Approve witness
        let approve = Witness::new(
            "trade_update_test".to_string(),
            "user_approver".to_string(),
            TimeStamp::new(),
            WitnessType::Approve,
        );
        ctx.insert_witness(approve);

        // State might be Approved or might be something else depending on witnesses
        // But after adding Update, it should definitely be PendingApproval
        let update = Witness::new(
            "trade_update_test".to_string(),
            "user_updater".to_string(),
            TimeStamp::new(),
            WitnessType::Update {
                details_hash: format!("hash_{}", update_hash),
            },
        );
        ctx.insert_witness(update);

        prop_assert_eq!(
            &ctx.current_state(),
            &TradeState::PendingApproval,
            "After Update (when not terminal), state should always be PendingApproval"
        );
    }
}
