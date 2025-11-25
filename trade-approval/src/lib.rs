//! # Trade Approval System
//! ![rustc-image](https://img.shields.io/badge/rustc-1.89-blue?logo=Rust)
//! ![license-image](https://img.shields.io/badge/license-MIT-green?logo=opensourceinitiative)
//!
//! A functional, content-addressable trade approval system for managing financial transactions
//! through an immutable witness chain, inspired by approaches to content-addressable storage.
//!
//! ## Overview
//!
//! This library provides a streamlined approval workflow for financial instruments. Rather than mutating records in a traditional database, the system treats
//! all data as immutable, content-addressable objects - similar to how Git stores commits and
//! trees. Trade state is derived by replaying an append-only witness chain, ensuring complete
//! auditability and eliminating state synchronisation bugs.
//!
//! ### Functional Architecture
//!
//! The system follows functional programming principles with clear separation of concerns:
//!
//! #### Content-Addressable Storage Layer
//!
//! All data objects (trade details, witnesses, contexts) are:
//! - **Immutable**: Once created, they never change
//! - **Content-addressed**: Identified by SHA256 hash of their CBOR-encoded contents
//! - **Self-describing**: Contains all information needed for verification
//! - **Persistent**: Stored in a key-value database (Sled)
//!
//! #### State Derivation Layer
//!
//! Trade state is **never stored**, only **derived** by walking the witness chain:
//! - `current_state()` replays witnesses to compute current state
//! - `requires_approval()` determines if approval is needed
//! - `get_expected_approver()` extracts who can approve
//!
//! This is analogous to Git determining the current working tree by replaying commits.
//!
//! #### Service Layer
//!
//! The [`service`] module acts as a stateless coordinator that:
//! - Loads immutable data from the content store
//! - Appends new witnesses to the chain
//! - Persists updated contexts back to storage
//! - Enforces business rules via state derivation
//!
//! ### Core Principles
//!
//! - **Immutability**: All trade data and workflow actions are immutable, content-addressable
//!   facts. Trades are never mutated; instead, new witnesses are appended to their history.
//! - **Derived State**: Trade state (e.g., `PendingApproval`, `Approved`) is never stored
//!   directly. Instead, it's an ephemeral value calculated by replaying the immutable
//!   witness history, eliminating state-synchronisation bugs.
//! - **Pure Validation Logic**: All business and workflow rules are concentrated in pure,
//!   stateless functions that can be tested exhaustively without external dependencies.
//!
//! ## Trade Lifecycle & State Machine
//!
//! Trade states are derived by walking the witness chain backward. The state machine includes:
//!
//! 1. **Draft**: No witnesses yet, trade details being constructed
//! 2. **PendingApproval**: Latest witness is `Submit` or `Update` - needs approval
//! 3. **Approved**: Latest witness is `Approve` with no subsequent `Update`
//! 4. **SentToExecute**: Trade sent to counterparty via `SendToExecute` witness
//! 5. **Booked**: Final witness is `Book` - trade is recorded in ledger
//! 6. **Cancelled**: `Cancel` witness terminates the trade lifecycle
//!
//! ### Witness Types & State Transitions
//!
//! Each witness type represents an immutable action appended to the chain:
//!
//! - **`Submit`**: Creates initial trade request (Draft → PendingApproval)
//!   - Contains: `details_hash`, `requester_id`, `approver_id`
//!   - Includes hash reference to immutable `TradeDetails` object
//!
//! - **`Approve`**: Approves current trade state (PendingApproval → Approved)
//!   - Verifies approver matches the one specified in `Submit`
//!   - Only valid if `current_state()` returns `PendingApproval`
//!
//! - **`Update`**: Modifies trade details (Approved → PendingApproval)
//!   - Contains: new `details_hash` pointing to updated details
//!   - Invalidates previous `Approve` witness - requires re-approval
//!   - Critical: This enables the re-approval workflow
//!
//! - **`SendToExecute`**: Sends to counterparty (Approved → SentToExecute)
//!   - Only valid if trade is in `Approved` state
//!
//! - **`Book`**: Records in ledger (SentToExecute → Booked)
//!   - Contains: `strike` price for final booking
//!
//! - **`Cancel`**: Terminates trade (Any → Cancelled)
//!   - Can occur at any point before `Booked`
//!
//! ## Validation Rules
//!
//! All trades must satisfy the following temporal constraint before submission:
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
//! ## Content-Addressable Storage: The Git Model
//!
//! The system uses a content-addressable store where objects are identified by the hash of
//! their contents.
//!
//! ### Storage Strategy
//!
//! ```text
//! Database (sled key-value store):
//! ┌─────────────────────────┬──────────────────────────────┐
//! │ Key                     │ Value (CBOR-encoded)         │
//! ├─────────────────────────┼──────────────────────────────┤
//! │ "trade_abc123"          │ TradeContext(with witnesses) │
//! │ sha256(trade_details_v1)│ TradeDetails v1              │
//! │ sha256(trade_details_v2)│ TradeDetails v2(after Update)│
//! └─────────────────────────┴──────────────────────────────┘
//! ```
//!
//! - **TradeContext**: Stored by `trade_id` (unhashed) for easy lookup
//! - **TradeDetails**: Stored by content hash for immutability and deduplication
//! - **Witnesses**: Embedded in `TradeContext.witness_set` as an append-only list
//!
//! ### Benefits
//!
//! - **Immutability**: Changed content produces a different hash/ID
//! - **Integrity**: Content cannot change without changing its ID
//! - **Deduplication**: Identical trade details share the same hash
//! - **Replay capability**: State reconstructed by walking witness chain
//! - **Auditability**: Complete history preserved, tamper-evident
//!
//! ## CBOR Encoding
//!
//! The library uses CBOR (Concise Binary Object Representation) for efficient
//! serialisation. This provides:
//!
//! - **Compact binary encoding** for network transmission and storage
//! - **Hash-safe encoding** for content-addressable storage
//! - **Type precision** without JSON's whitespace and base64 inflation
//! - **Deterministic byte sequences** for consistent hashing
//!
//! Note: Canonical CBOR is not enforced; standard CBOR encoding is used by default.
//!
//! ## Example Workflow
//!
//! ### Basic Approval Flow
//!
//! Note: IDs are made to be human-readable for the purposes of this example
//!
//! ```rust,ignore
//! use trade_approval::service::TradeService;
//! use trade_approval::trade::{TradeDetails, Currency, Direction, TimeStamp};
//! use std::sync::Arc;
//!
//! // Initialize the service with sled database
//! let db = Arc::new(sled::open("trade_db")?);
//! let service = TradeService::new(db);
//!
//! // 1. Build trade details using the builder pattern
//! let trade_details = TradeDetails::new()
//!     .new_trade_entity("entity_abc")
//!     .new_counter_party("counterparty_xyz")
//!     .set_direction(Direction::Buy)
//!     .set_notional_currency(Currency::USD)
//!     .set_notional_amount(1_000_000)
//!     .set_underlying_currency(Currency::EUR)
//!     .set_underlying_amount(850_000)
//!     .set_trade_date(TimeStamp::new())
//!     .set_value_date(TimeStamp::new())
//!     .set_delivery_date(TimeStamp::new());
//!
//! // 2. Submit trade for approval (creates Submit witness → PendingApproval)
//! let trade_ctx = service.submit_trade(
//!     trade_details,
//!     "requester_user123".to_string(),
//!     "approver_user456".to_string(),
//!     "user_addr_123".to_string(),
//! )?;
//!
//! println!("Trade submitted: {}", trade_ctx.trade_id);
//! println!("Current state: {:?}", trade_ctx.current_state()); // PendingApproval
//!
//! // 3. Approve the trade (creates Approve witness → Approved)
//! let approved_ctx = service.approve_trade(
//!     trade_ctx.trade_id.clone(),
//!     "approver_user456".to_string(),
//! )?;
//!
//! println!("Current state: {:?}", approved_ctx.current_state()); // Approved
//!
//! // 4. Execute the approved trade (creates SendToExecute witness → SentToExecute)
//! let executed_ctx = service.execute_trade(
//!     trade_ctx.trade_id.clone(),
//!     "user_addr_123".to_string(),
//! )?;
//!
//! // 5. Book the executed trade (creates Book witness → Booked)
//! let booked_ctx = service.book_trade(
//!     trade_ctx.trade_id.clone(),
//!     "user_addr_123".to_string(),
//!     85000, // strike price
//! )?;
//! ```
//!
//! ### Re-Approval After Update Flow
//!
//! ```rust,ignore
//! // After initial approval, trader realizes they need to change the amount
//! let updated_details = TradeDetails::new()
//!     .new_trade_entity("entity_abc")
//!     .new_counter_party("counterparty_xyz")
//!     .set_direction(Direction::Buy)
//!     .set_notional_currency(Currency::USD)
//!     .set_notional_amount(1_500_000) // CHANGED!
//!     .set_underlying_currency(Currency::EUR)
//!     .set_underlying_amount(1_275_000) // CHANGED!
//!     .set_trade_date(TimeStamp::new())
//!     .set_value_date(TimeStamp::new())
//!     .set_delivery_date(TimeStamp::new());
//!
//! // Update the trade (creates Update witness → PendingApproval again)
//! let updated_ctx = service.update_trade(
//!     trade_ctx.trade_id.clone(),
//!     updated_details,
//!     "user_addr_123".to_string(),
//! )?;
//!
//! // State is now PendingApproval because Update invalidated previous Approve
//! println!("After update: {:?}", updated_ctx.current_state()); // PendingApproval
//! println!("Requires approval: {}", updated_ctx.requires_approval()); // true
//!
//! // Need to approve again before execution
//! let reapproved_ctx = service.approve_trade(
//!     trade_ctx.trade_id.clone(),
//!     "approver_user456".to_string(),
//! )?;
//!
//! // Now approved again and ready for execution
//! println!("Re-approved: {:?}", reapproved_ctx.current_state()); // Approved
//! ```
//!
//! ### Understanding the Witness Chain
//!
//! After the re-approval flow above, the witness chain looks like:
//!
//! ```text
//! [Submit] → [Approve] → [Update] → [Approve]
//!                           ↑
//!                     This Update invalidates
//!                     the previous Approve,
//!                     requiring re-approval
//!
//! Walking backward from the end:
//! - Last witness is Approve → state is Approved
//! - If last witness was Update → state would be PendingApproval
//! - The Update witness contains a new details_hash pointing to v2 of trade details
//! ```
//!
//! ## Witnesses: The Immutable Event Chain
//!
//! Witnesses are the atomic units of change in the system - immutable records of actions
//! appended to a trade's history, analogous to commits:
//!
//! ### Witness Structure
//!
//! Each witness contains:
//! - **`trade_id`**: Reference to the parent `TradeContext` (Acts as our primary ref)
//! - **`user_id`**: The actor who created this witness
//! - **`user_timestamp`**: When the action occurred
//! - **`witness_type`**: The action performed with its data payload
//!
//! ### Creating a Trade: The Functional Flow
//!
//! 1. **Generate immutable trade_id** (uuid7-based, bech32-encoded)
//! 2. **Create TradeContext** with empty witness_set
//! 3. **Validate & hash TradeDetails** → store in DB
//! 4. **Create Submit witness** with details_hash → append to witness_set
//! 5. **Persist TradeContext** to DB using trade_id as key
//!
//! All subsequent actions follow the same pattern: load context, append witness, persist.
//! The witness chain is append-only - never modified, only extended.
//!
//! ## Design Goals & Philosophy
//!
//! This system embraces functional programming principles and content-addressable storage:
//!
//! - **Immutability First**: No data is ever mutated. Changes create new versions with new hashes.
//!   This eliminates entire classes of bugs related to concurrent modifications and state races.
//!
//! - **Derived State**: Trade state is computed from the witness chain, not stored. This means
//!   the state machine logic is centralised in `current_state()` rather than scattered across
//!   mutations.
//!
//! - **Append-Only History**: Witnesses are only added, never changed or deleted.
//!   This provides complete auditability and makes the system naturally event-sourced.
//!
//! - **Content Addressing**: Objects are identified by their content hash. This makes them
//!   self-validating, naturally deduplicated, and tamper-evident.
//!
//! - **Stateless Service**: The `TradeService` has no state of its own - it's purely a
//!   coordinator that loads data, applies business rules, appends witnesses, and persists.
//!   This makes it trivially horizontally scalable.
//!
//! - **Type Safety**: Rust's type system prevents invalid states at compile time. The witness
//!   types are distinct enum variants, making it impossible to confuse a Submit with an Approve.
//!
//! - **Testability**: State derivation logic is pure functions with no I/O dependencies,
//!   enabling comprehensive unit testing without mocking databases or networks.
//!
//!## License
//!
//! This crate is licensed under:
//!
//!  * [MIT license](https://opensource.org/licenses/MIT)

pub mod context;
pub mod error;
pub mod service;
pub mod trade;
pub mod utils;
