#![allow(unused_imports)]

use anyhow::Context;
use sled::open;
use std::sync::Arc;
use trade_approval::{context, service::TradeService, trade, utils};

use tempfile::tempdir; // Use for test db cleanup.

#[test]
fn submit_and_approve_trade() -> anyhow::Result<()> {
    // Sled uses file-based locking to prevent concurrent access, so only one test
    // can hold the lock at a time. As is good practice in testing create separate
    // databases for each test. The db is created on temp for simplified cleanup.
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_submit_and_approve.db");
    let db = open(db_path)?;
    let db = Arc::new(db);

    // reset the db for each test run
    db.clear()?;

    // create a new service instance
    let service = TradeService::new(db);

    let requester_id = utils::new_uuid_to_bech32("user_")?;
    let approver_id = utils::new_uuid_to_bech32("user_")?;
    let user_id = utils::new_uuid_to_bech32("user_")?;
    let trade_entity = utils::new_uuid_to_bech32("user_")?;
    let counter_party = utils::new_uuid_to_bech32("user_")?;
    // keeping it the same time for now
    let timestamp = trade::TimeStamp::new();

    let trade_details = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Buy)
        .set_notional_amount(20_000)
        .set_underlying_amount(15_000)
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp);

    let ctx = service
        .submit_trade(trade_details, requester_id, approver_id.clone(), user_id)
        .context("Trade Failed on Submit: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::PendingApproval);

    // with our trade submitted we can move onto the next step, approval

    let ctx = service
        .approve_trade(ctx.trade_id.clone(), approver_id)
        .context("Trade Failed on Approval: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::Approved);

    Ok(())
}

#[test]
fn update_trade() -> anyhow::Result<()> {
    // Sled uses file-based locking to prevent concurrent access, so only one test
    // can hold the lock at a time. As is good practice in testing create separate
    // databases for each test. The db is created on temp for simplified cleanup.
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("test_update_trade.db");
    let db = open(db_path)?;
    let db = Arc::new(db);

    db.clear()?;

    // same as before
    let service = TradeService::new(db);

    let requester_id = utils::new_uuid_to_bech32("user_")?;
    let approver_id = utils::new_uuid_to_bech32("user_")?;
    let user_id = utils::new_uuid_to_bech32("user_")?;
    let trade_entity = utils::new_uuid_to_bech32("user_")?;
    let counter_party = utils::new_uuid_to_bech32("user_")?;
    // keeping it the same time for now
    let timestamp = trade::TimeStamp::new();

    let trade_details = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Buy)
        .set_notional_amount(20_000)
        .set_underlying_amount(15_000)
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp.clone());

    let ctx = service
        .submit_trade(
            trade_details,
            requester_id,
            approver_id.clone(),
            user_id.clone(),
        )
        .context("Trade Failed on Submit: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::PendingApproval);

    let ctx = service
        .approve_trade(ctx.trade_id.clone(), approver_id)
        .context("Trade Failed on Approval: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::Approved);

    let trade_details_update = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Sell) // changed
        .set_notional_amount(10_000) // changed
        .set_underlying_amount(5_000) // changed
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp);

    let ctx = service.update_trade(ctx.trade_id, trade_details_update, user_id)?;

    assert_eq!(ctx.current_state(), context::TradeState::PendingApproval);

    Ok(())
}

#[test]
fn submit_execute_and_book() -> anyhow::Result<()> {
    // Sled uses file-based locking to prevent concurrent access, so only one test
    // can hold the lock at a time. As is good practice in testing create separate
    // databases for each test. The db is created on temp for simplified cleanup.
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("submit_execute_and_book.db");
    let db = open(db_path)?;
    let db = Arc::new(db);

    // reset the db for each test run
    db.clear()?;

    let service = TradeService::new(db);

    let requester_id = utils::new_uuid_to_bech32("user_")?;
    let approver_id = utils::new_uuid_to_bech32("user_")?;
    let user_id = utils::new_uuid_to_bech32("user_")?;
    let trade_entity = utils::new_uuid_to_bech32("user_")?;
    let counter_party = utils::new_uuid_to_bech32("user_")?;
    let timestamp = trade::TimeStamp::new();

    let trade_details = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Buy)
        .set_notional_amount(20_000)
        .set_underlying_amount(15_000)
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp);

    let ctx = service
        .submit_trade(
            trade_details,
            requester_id,
            approver_id.clone(),
            user_id.clone(),
        )
        .context("Trade Failed on Submit: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::PendingApproval);

    // with our trade submitted we can move onto the next step, approval

    let ctx = service
        .approve_trade(ctx.trade_id.clone(), approver_id)
        .context("Trade Failed on Approval: ")?;

    assert_eq!(ctx.current_state(), context::TradeState::Approved);

    let ctx = service.execute_trade(ctx.trade_id, user_id.clone())?;

    assert_eq!(ctx.current_state(), context::TradeState::SentToExecute);

    let ctx = service.book_trade(ctx.trade_id, user_id, 1_000_040)?;
    assert_eq!(ctx.current_state(), context::TradeState::Booked);

    Ok(())
}

#[test]
fn view_history() -> anyhow::Result<()> {
    // Sled uses file-based locking to prevent concurrent access, so only one test
    // can hold the lock at a time. As is good practice in testing create separate
    // databases for each test. The db is created on temp for simplified cleanup.
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("submit_execute_and_book.db");
    let db = open(db_path)?;
    let db = Arc::new(db);

    // reset the db for each test run
    db.clear()?;

    let service = TradeService::new(db);

    let requester_id = utils::new_uuid_to_bech32("user_")?;
    let approver_id = utils::new_uuid_to_bech32("user_")?;
    let user_id = utils::new_uuid_to_bech32("user_")?;
    let trade_entity = utils::new_uuid_to_bech32("user_")?;
    let counter_party = utils::new_uuid_to_bech32("user_")?;
    let timestamp = trade::TimeStamp::new();

    let trade_details = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Buy)
        .set_notional_amount(20_000)
        .set_underlying_amount(15_000)
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp.clone());

    let ctx = service
        .submit_trade(
            trade_details,
            requester_id,
            approver_id.clone(),
            user_id.clone(),
        )
        .context("Trade Failed on Submit: ")?;
    let ctx = service
        .approve_trade(ctx.trade_id.clone(), approver_id.clone())
        .context("Trade Failed on Approval: ")?;

    let trade_details = trade::TradeDetails::new()
        .new_trade_entity(trade_entity.as_ref())
        .new_counter_party(counter_party.as_ref())
        .set_notional_currency(trade::Currency::USD)
        .set_direction(trade::Direction::Buy)
        .set_notional_amount(25_000)
        .set_underlying_amount(15_000)
        .set_underlying_currency(trade::Currency::GBP)
        .set_trade_date(timestamp.clone())
        .set_delivery_date(timestamp.clone())
        .set_value_date(timestamp);
    let ctx = service.update_trade(ctx.trade_id.clone(), trade_details, user_id.clone())?;
    let ctx = service.approve_trade(ctx.trade_id.clone(), approver_id)?;
    let ctx = service.execute_trade(ctx.trade_id, user_id.clone())?;
    let ctx = service.book_trade(ctx.trade_id, user_id, 25_000)?;

    assert!(!ctx.witness_set.is_empty());

    ctx.view_history();

    Ok(())
}
