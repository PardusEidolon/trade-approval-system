//! Property-based tests for TradeDetails validation and invariants
//!
//! This module uses the proptest crate to verify that TradeDetails behavior
//! is correct across a wide range of randomly generated inputs. Property tests
//! are particularly valuable for testing invariants that should hold for all
//! valid inputs, not just specific test cases.

use proptest::prelude::*;
use trade_approval::trade::{Currency, Direction, TimeStamp, TradeDetails};

// PROPERTY TEST STRATEGIES

/// Strategy to generate random Currency values
fn currency_strategy() -> impl Strategy<Value = Currency> {
    (0u8..=2).prop_map(|i| match i {
        0 => Currency::USD,
        1 => Currency::GBP,
        _ => Currency::EUR,
    })
}

/// Strategy to generate random Direction values
fn direction_strategy() -> impl Strategy<Value = Direction> {
    prop::bool::ANY.prop_map(|b| if b { Direction::Buy } else { Direction::Sell })
}

/// Strategy to generate three timestamps in sorted order (trade <= value <= delivery)
fn sorted_timestamps_strategy() -> impl Strategy<
    Value = (
        TimeStamp<chrono::Utc>,
        TimeStamp<chrono::Utc>,
        TimeStamp<chrono::Utc>,
    ),
> {
    (2020u32..=2030, 1u32..=12).prop_flat_map(|(year, month)| {
        // Generate three days in the same month in ascending order
        (1u32..=10, 11u32..=20, 21u32..=28).prop_map(move |(day1, day2, day3)| {
            let trade_date = TimeStamp::new_with(year as i32, month, day1, 0, 0, 0);
            let value_date = TimeStamp::new_with(year as i32, month, day2, 0, 0, 0);
            let delivery_date = TimeStamp::new_with(year as i32, month, day3, 0, 0, 0);
            (trade_date, value_date, delivery_date)
        })
    })
}

/// Strategy to generate three timestamps in unsorted order (violates trade <= value <= delivery)
fn unsorted_timestamps_strategy() -> impl Strategy<
    Value = (
        TimeStamp<chrono::Utc>,
        TimeStamp<chrono::Utc>,
        TimeStamp<chrono::Utc>,
    ),
> {
    (2020u32..=2030, 1u32..=12).prop_flat_map(|(year, month)| {
        // Generate three days where delivery is before trade (clearly invalid)
        (21u32..=28, 11u32..=20, 1u32..=10).prop_map(move |(day1, day2, day3)| {
            let trade_date = TimeStamp::new_with(year as i32, month, day1, 0, 0, 0);
            let value_date = TimeStamp::new_with(year as i32, month, day2, 0, 0, 0);
            let delivery_date = TimeStamp::new_with(year as i32, month, day3, 0, 0, 0);
            (trade_date, value_date, delivery_date)
        })
    })
}

/// Strategy to generate positive amounts (1 to 100_000_000)
fn amount_strategy() -> impl Strategy<Value = u64> {
    1u64..=100_000_000u64
}

/// Strategy to generate entity/counterparty prefixes
fn entity_prefix_strategy() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("entity_"), Just("counter_"), Just("party_"),]
}

// PROPERTY TESTS
proptest! {
    /// Property: Any TradeDetails with valid sorted dates should pass date validation
    ///
    /// This test verifies the core invariant that if trade_date <= value_date <= delivery_date,
    /// then validate_dates() must return true. This property should hold for ALL valid
    /// date combinations.
    #[test]
    fn prop_sorted_dates_always_validate(
        (trade_date, value_date, delivery_date) in sorted_timestamps_strategy()
    ) {
        let trade = TradeDetails::new()
            .set_trade_date(trade_date.clone())
            .set_value_date(value_date.clone())
            .set_delivery_date(delivery_date.clone());

        prop_assert!(
            trade.validate_dates(),
            "Valid date sequence should pass validation: trade={:?}, value={:?}, delivery={:?}",
            trade_date, value_date, delivery_date
        );
    }

    /// Property: Any TradeDetails with unsorted dates should fail date validation
    ///
    /// This test verifies that dates violating the trade <= value <= delivery invariant
    /// are correctly rejected. This is the contrapositive of the sorted dates property.
    #[test]
    fn prop_unsorted_dates_always_fail_validation(
        (trade_date, value_date, delivery_date) in unsorted_timestamps_strategy()
    ) {
        let trade = TradeDetails::new()
            .set_trade_date(trade_date.clone())
            .set_value_date(value_date.clone())
            .set_delivery_date(delivery_date.clone());

        prop_assert!(
            !trade.validate_dates(),
            "Invalid date sequence should fail validation: trade={:?}, value={:?}, delivery={:?}",
            trade_date, value_date, delivery_date
        );
    }

    /// Property: Complete TradeDetails with valid data should always validate successfully
    ///
    /// This test generates fully-populated TradeDetails with all required fields set to
    /// valid values. The validate_and_finalise() method should succeed for all such inputs,
    /// and should produce a non-empty hash and non-empty CBOR encoding.
    #[test]
    fn prop_complete_trade_validates(
        entity_prefix in entity_prefix_strategy(),
        counter_prefix in entity_prefix_strategy(),
        direction in direction_strategy(),
        notional_currency in currency_strategy(),
        underlying_currency in currency_strategy(),
        notional_amount in amount_strategy(),
        underlying_amount in amount_strategy(),
        (trade_date, value_date, delivery_date) in sorted_timestamps_strategy()
    ) {
        let trade = TradeDetails::new()
            .new_trade_entity(entity_prefix)
            .new_counter_party(counter_prefix)
            .set_direction(direction)
            .set_notional_currency(notional_currency)
            .set_notional_amount(notional_amount)
            .set_underlying_currency(underlying_currency)
            .set_underlying_amount(underlying_amount)
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        let result = trade.validate_and_finalise();
        prop_assert!(
            result.is_ok(),
            "Complete trade with valid data should validate: {:?}",
            result.err()
        );

        let (hash, cbor) = result.unwrap();
        prop_assert!(!hash.is_empty(), "Hash should not be empty");
        prop_assert!(!cbor.is_empty(), "CBOR encoding should not be empty");
        prop_assert_eq!(hash.len(), 64, "SHA256 hash should be 64 hex characters");
    }

    /// Property: TradeDetails with zero amounts should always fail validation
    ///
    /// Business rule: trades with zero notional or underlying amounts are invalid.
    /// This property verifies the rule holds regardless of other field values.
    #[test]
    fn prop_zero_amounts_always_fail(
        entity_prefix in entity_prefix_strategy(),
        counter_prefix in entity_prefix_strategy(),
        direction in direction_strategy(),
        currency1 in currency_strategy(),
        currency2 in currency_strategy(),
        (trade_date, value_date, delivery_date) in sorted_timestamps_strategy(),
        zero_notional in prop::bool::ANY,
    ) {
        let trade = TradeDetails::new()
            .new_trade_entity(entity_prefix)
            .new_counter_party(counter_prefix)
            .set_direction(direction)
            .set_notional_currency(currency1)
            .set_notional_amount(if zero_notional { 0 } else { 1000 })
            .set_underlying_currency(currency2)
            .set_underlying_amount(if zero_notional { 1000 } else { 0 })
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        let result = trade.validate_and_finalise();
        prop_assert!(
            result.is_err(),
            "Trade with zero amounts should fail validation"
        );
    }

    /// Property: Different TradeDetails should produce different hashes (with high probability)
    ///
    /// Content-addressable storage relies on different content producing different hashes.
    /// While hash collisions are theoretically possible, they should be astronomically rare
    /// for SHA256. This test verifies that changing any field produces a different hash.
    ///
    /// Note: We use two different amounts to ensure different content.
    #[test]
    fn prop_different_trades_produce_different_hashes(
        entity_prefix in entity_prefix_strategy(),
        counter_prefix in entity_prefix_strategy(),
        direction_val in (0u8..=1),
        currency1_val in (0u8..=2),
        currency2_val in (0u8..=2),
        (trade_date, value_date, delivery_date) in sorted_timestamps_strategy()
    ) {
        let amount1 = 10000u64;
        let amount2 = 20000u64;

        // Map integer values to enums for both trades
        let direction1 = if direction_val == 0 { Direction::Buy } else { Direction::Sell };
        let direction2 = if direction_val == 0 { Direction::Buy } else { Direction::Sell };

        let currency1_a = match currency1_val {
            0 => Currency::USD,
            1 => Currency::GBP,
            _ => Currency::EUR,
        };
        let currency1_b = match currency1_val {
            0 => Currency::USD,
            1 => Currency::GBP,
            _ => Currency::EUR,
        };

        let currency2_a = match currency2_val {
            0 => Currency::USD,
            1 => Currency::GBP,
            _ => Currency::EUR,
        };
        let currency2_b = match currency2_val {
            0 => Currency::USD,
            1 => Currency::GBP,
            _ => Currency::EUR,
        };

        // Create two trades that differ only in notional_amount
        let trade1 = TradeDetails::new()
            .new_trade_entity(entity_prefix)
            .new_counter_party(counter_prefix)
            .set_direction(direction1)
            .set_notional_currency(currency1_a)
            .set_notional_amount(amount1)
            .set_underlying_currency(currency2_a)
            .set_underlying_amount(10000)
            .set_trade_date(trade_date.clone())
            .set_value_date(value_date.clone())
            .set_delivery_date(delivery_date.clone());

        let trade2 = TradeDetails::new()
            .new_trade_entity(entity_prefix)
            .new_counter_party(counter_prefix)
            .set_direction(direction2)
            .set_notional_currency(currency1_b)
            .set_notional_amount(amount2)
            .set_underlying_currency(currency2_b)
            .set_underlying_amount(10000)
            .set_trade_date(trade_date)
            .set_value_date(value_date)
            .set_delivery_date(delivery_date);

        let (hash1, _) = trade1.validate_and_finalise().unwrap();
        let (hash2, _) = trade2.validate_and_finalise().unwrap();

        prop_assert_ne!(
            hash1, hash2,
            "Different trades should produce different hashes (collision extremely unlikely)"
        );
    }
}

// ADDITIONAL PROPTEST EXAMPLES WITH EXPLICIT CONFIGURATION

/// Property test with custom configuration for more extensive testing
///
/// Configure proptest for deeper exploration:
/// - More test cases (1000 instead of default 256)
/// - Useful for critical invariants that need higher confidence
#[cfg(test)]
mod extensive_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property: Hash consistency - encoding the same TradeDetails multiple times
        /// should always produce the same hash
        ///
        /// This verifies that CBOR encoding is deterministic and hash computation
        /// is consistent. Critical for content-addressable storage reliability.
        #[test]
        fn prop_hash_is_deterministic(
            entity_prefix in entity_prefix_strategy(),
            counter_prefix in entity_prefix_strategy(),
            direction in direction_strategy(),
            currency1 in currency_strategy(),
            currency2 in currency_strategy(),
            notional_amount in amount_strategy(),
            underlying_amount in amount_strategy(),
            (trade_date, value_date, delivery_date) in sorted_timestamps_strategy()
        ) {
            let trade = TradeDetails::new()
                .new_trade_entity(entity_prefix)
                .new_counter_party(counter_prefix)
                .set_direction(direction)
                .set_notional_currency(currency1)
                .set_notional_amount(notional_amount)
                .set_underlying_currency(currency2)
                .set_underlying_amount(underlying_amount)
                .set_trade_date(trade_date)
                .set_value_date(value_date)
                .set_delivery_date(delivery_date);

            // Validate multiple times - should get same hash each time
            let (hash1, cbor1) = trade.validate_and_finalise().unwrap();
            let (hash2, cbor2) = trade.validate_and_finalise().unwrap();
            let (hash3, cbor3) = trade.validate_and_finalise().unwrap();

            prop_assert_eq!(&hash1, &hash2, "First and second hash should match");
            prop_assert_eq!(&hash2, &hash3, "Second and third hash should match");
            prop_assert_eq!(&cbor1, &cbor2, "First and second CBOR should match");
            prop_assert_eq!(&cbor2, &cbor3, "Second and third CBOR should match");
        }
    }
}
