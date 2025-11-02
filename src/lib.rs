//! # Trade Approval System
//!
//! A type-safe, witness-based trade approval system for managing transactions through a structured validation and approval workflow.
//!
//! ## Overview
//!
//! This library provides a streamlined and standardized approval process for financial
//! instruments, particularly forward contracts. It enforces immutability, maintains
//! complete audit trails through witness events, and ensures type-level guarantees
//! about transaction states.
//!
//! ### Two-Layer Architecture
//!
//! The system is cleanly separated into two distinct layers:
//!
//! #### Layer 1: The Pure Logic Core
//!
//! A stateless module containing all business and workflow logic with no knowledge of
//! databases or I/O. This layer provides:
//!
//! - **`validate_for_execution`**: The holistic validator that accepts trade details and
//!   their complete witness history to determine if a trade is valid for execution. This
//!   provides "off-chain" validation capability without database access.
//! - **`derive_state`**: The state derivation function that replays the witness list to
//!   determine the current status of a trade for presentation purposes.
//!
//! #### Layer 2: The Service & API Layer
//!
//! The imperative "shell" that orchestrates I/O and state persistence through the
//! [`service`] module. This layer acts as a **"Witness Collector"**, performing minimal
//! logic and primarily creating and persisting witness objects for user actions.
//!
//! ### Core Principles
//!
//! - **Immutability**: All trade data and workflow actions are immutable, content-addressable
//!   facts. Trades are never mutated; instead, new witnesses are appended to their history.
//! - **Derived State**: Trade state (e.g., `PendingApproval`, `Approved`) is never stored
//!   directly. Instead, it's an ephemeral value calculated by replaying the immutable
//!   witness history, eliminating state-synchronization bugs.
//! - **Pure Validation Logic**: All business and workflow rules are concentrated in pure,
//!   stateless functions that can be tested exhaustively without external dependencies.
//!
//! ## Trade Lifecycle
//!
//! Transactions progress through the following states:
//!
//! 1. **Draft**: Trades are constructed using the builder pattern
//! 2. **PendingApproval**: Submitted trades await approval
//! 3. **NeedsReApproval**: Updated trades require re-approval
//! 4. **Approved**: Approved trades ready to be sent to counterparty
//! 5. **SentToCounterParty**: Trade sent and awaiting execution
//! 6. **Executed**: Successfully executed and ready to book
//! 7. **Cancelled**: Trade cancelled (can occur at any point before execution)
//!
//! ### State Transitions
//!
//! - **Submit**: Add trade to approval queue
//! - **Update**: Modify trade details (returns to approval queue)
//! - **Approve**: Move trade to execution stage
//! - **Execute**: Send finalized trade to counterparty
//! - **Book**: Record executed trade in ledger
//! - **Cancel**: Abort trade before execution
//!
//! ## Validation Rules
//!
//! All trades must satisfy the following temporal constraint before Submition:
//!
//! ```text
//! trade_date <= value_date <= delivery_date
//! ```
//!
//! Additional validation includes:
//! - Proper witness signatures at each stage
//! - Re-approval after updates
//! - Prevention of double execution
//! - Cancellation detection
//!
//! ## Content-Addressable Storage
//!
//! Trade details and witnesses use content-addressable storage where each object's ID is
//! the hash of its contents. This guarantees:
//!
//! - **Immutability**: Changed content produces a different hash/ID
//! - **Deduplication**: Identical content has identical IDs
//! - **Data integrity**: Content cannot change without changing its ID
//! - **Replay capability**: State can be reconstructed from witness history
//!
//! ## CBOR Encoding
//!
//! The library uses CBOR (Concise Binary Object Representation) for efficient
//! serialization. This provides:
//!
//! - **Compact binary encoding** for network transmission and storage
//! - **Hash-safe encoding** for content-addressable storage
//! - **Type precision** without JSON's whitespace and base64 inflation
//! - **Deterministic byte sequences** for consistent hashing
//!
//! Note: Canonical CBOR is not enforced; standard CBOR encoding is used by default.
//!
//! ## Modules
//!
//! - [`context`]: Trade context and witness management for state derivation
//! - [`error`]: Validation and operational error types
//! - [`service`]: Service layer API for trade workflow operations
//! - [`trade`]: Core trade details and witness types
//! - [`utils`]: Utility functions for hashing and serialization
//!
//! ## Example Workflow
//!
//! ```rust,ignore
//! use validus_trade_approval::service::TradeService;
//! use validus_trade_approval::trade::{TradeDetails, Currency, Direction};
//!
//! // Initialize the service
//! let service = TradeService::new()?;
//!
//! // Create wallets for the participants
//! let requester = service.create_wallet("Alice".to_string()).await?;
//! let approver = service.create_wallet("Bob".to_string()).await?;
//!
//! // Create trade details
//! let details = TradeDetails {
//!     trading_entity: requester.address.clone(),
//!     counterparty: "CounterParty Corp".to_string(),
//!     direction: Direction::Buy,
//!     notional_currency: Currency::USD,
//!     notional_amount: 1_000_000,
//!     // ... other fields
//! };
//!
//! // Submit trade (creates Submit witness)
//! let trade = service.submit_trade(
//!     requester.address.clone(),
//!     approver.address.clone(),
//!     details
//! ).await?;
//!
//! // Approve trade (creates Approve witness)
//! service.approve_trade(approver.address.clone(), trade.id).await?;
//!
//! // Validate and execute (calls pure validator, creates SendToExecute witness)
//! service.send_to_execute(requester.address, trade.id).await?;
//! ```
//!
//! ## Witnesses: The Event Chain
//!
//! Witnesses are immutable, content-addressable records of actions taken on a trade. Each
//! witness contains:
//!
//! - **`trade_id`**: A reference to the parent `TradeContext` (the stable lifecycle tracker).
//!   When a new trade is created, the `trade_id` is generated first, then all witnesses for
//!   that trade reference this same ID.
//! - **`user_address`**: The "signer" of the witness (serves as a signature)
//! - **`timestamp_utc`**: When the action occurred
//! - **`witness_type`**: The type of action (Submit, Approve, Update, Cancel, etc.)
//!
//! ### Creating a New Trade
//!
//! 1. Generate a stable `trade_id` (uuid7-based, bech32-encoded)
//! 2. Create a `TradeContext` with this `trade_id`
//! 3. Create the first `Submit` witness that references this `trade_id` and contains the
//!    hash of the `TradeDetails`
//! 4. All subsequent witnesses (Approve, Update, Cancel, etc.) reference the same `trade_id`
//!
//! The complete ordered list of witnesses for a trade constitutes its full history. By
//! replaying this history, the system can derive the current state and validate whether
//! the trade is ready for execution.
//!
//! ## Design Goals
//!
//! - **Simplicity & Testability**: Pure functional core enables exhaustive testing without
//!   external dependencies, critical for meeting tight development timelines
//! - **Type Safety**: Strong typing ensures methods are only available in valid states
//! - **Complete Audit Trail**: Immutable witness chain provides full event history
//! - **Offline Validation**: Validate correctness before network/database interaction
//! - **Content Addressable**: Trade details identified by hash, guaranteeing immutability
//! - **Replay Capability**: State can be reconstructed from event history at any time
//! - **Separation of Concerns**: Pure logic core isolated from imperative I/O shell

#![allow(unused_imports)]

pub mod context;
pub mod error;
pub mod service;
pub mod trade;
pub mod utils;

#[cfg(test)]
mod tests {
    use super::*;
}
