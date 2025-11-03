# Trade Approval System

A functional, content-addressable trade approval system for financial transactions built with Rust.

## Overview

This system manages the lifecycle of forward contract trades through an approval workflow. Rather than mutating database records, it treats all data as immutable, content-addressed objects. Trade state is derived by replaying an append-only witness chain.

**Key Features:**
- **Immutable by design** - all data is content-addressed and tamper-evident
- **Complete audit trail** - append-only witness chain records every action
- **Derived state** - no stored state, computed from witness history
- **Re-approval workflow** - updates invalidate previous approvals
- **Pure functional core** - state derivation logic has zero I/O dependencies

## Quick Start

### Prerequisites

**Using Nix + devenv (recommended):**
- [Nix installer](https://determinate.systems/nix-installer/)
- [devenv](https://devenv.sh/getting-started/)
- [direnv](https://direnv.net/)

**Or install Rust directly:**
- [rustup](https://rustup.rs/)

### Build & Test

```bash
# Build the project
cargo build

# Run tests
cargo test

# With nextest (if installed)
cargo nextest run --lib --success-output=immediate

# Generate documentation
cargo doc --open
```

## Trade Lifecycle

The trade lifecycle is modeled as a Petri net, shown below:

![Trade State Machine](./img/trade_formalisation_petri.png)

**Reading the Petri Net:**
- **Circles** = States (e.g., Draft, PendingApproval, Approved)
- **Rectangles** = Transitions/Actions (e.g., Submit, Approve, Update)
- **Arrows** = Flow direction
- **Tokens (dots)** = Current state position

Each transition creates an immutable witness appended to the chain. See the [full documentation](#documentation) for detailed explanation of the state machine and witness types.

## Usage Example

```rust
use trade_approval::service::TradeService;
use trade_approval::trade::{TradeDetails, Currency, Direction, TimeStamp};
use std::sync::Arc;

// Initialize the service with sled database
let db = Arc::new(sled::open("trade_db")?);
let service = TradeService::new(db);

// 1. Build trade details using the builder pattern
let trade_details = TradeDetails::new()
    .new_trade_entity("entity_abc")
    .new_counter_party("counterparty_xyz")
    .set_direction(Direction::Buy)
    .set_notional_currency(Currency::USD)
    .set_notional_amount(1_000_000)
    .set_underlying_currency(Currency::EUR)
    .set_underlying_amount(850_000)
    .set_trade_date(TimeStamp::new())
    .set_value_date(TimeStamp::new())
    .set_delivery_date(TimeStamp::new());

// 2. Submit trade for approval (creates Submit witness → PendingApproval)
let trade_ctx = service.submit_trade(
    trade_details,
    "requester_user123".to_string(),
    "approver_user456".to_string(),
    "user_addr_123".to_string(),
)?;

println!("Trade submitted: {}", trade_ctx.trade_id);
println!("Current state: {:?}", trade_ctx.current_state()); // PendingApproval

// 3. Approve the trade (creates Approve witness → Approved)
let approved_ctx = service.approve_trade(
    trade_ctx.trade_id.clone(),
    "approver_user456".to_string(),
)?;

println!("Current state: {:?}", approved_ctx.current_state()); // Approved
```

## Documentation

**For comprehensive documentation, architecture details, and complete examples, please refer to the Rust documentation:**

```bash
cargo doc --open
```

The documentation includes:
- Detailed architecture and design philosophy
- Complete witness type specifications
- State machine mechanics and derivation logic
- Full workflow examples (basic approval, re-approval, cancellation)
- Content-addressable storage strategy
- API reference for all modules

## License

MIT
