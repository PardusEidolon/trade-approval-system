# Architecture Decision Records (ADR)

## Content-Addressable Storage Model

**Context:**
Trade approval systems traditionally use mutable database records where state is updated in-place. This approach leads to several challenges:
- Loss of audit trail when records are overwritten
- State synchronization bugs between different components
- Difficulty in reconstructing historical state
- Lack of tamper evidence

**Decision:**
Adopt a content-addressable storage model inspired by Git and UTXO systems, where all data objects are:
- Immutable after creation
- Identified by SHA256 hash of their CBOR-encoded contents
- Self-describing and self-validating

**Rationale:**
- **Auditability**: Complete history is preserved automatically since nothing is ever deleted or modified
- **Integrity**: Content cannot change without changing its identifier, making tampering evident
- **Deduplication**: Identical trade details share the same hash, reducing storage overhead
- **Simplicity**: No need for complex state synchronization logic - state is always derived from history

**Trade-Offs:**
- Positive: Complete audit trail by design, tamper-evident storage, simplified state management
- Negative: Increased storage requirements (all versions retained), requires hash computation for lookups
- Trade-off: Storage cost for operational simplicity and auditability

---

## Derived State via Witness Chain

**Context:**
Most workflow systems store the current state explicitly in database fields (e.g., `status = 'APPROVED'`). This creates the dual responsibility of maintaining both the event history and the current state, leading to potential inconsistencies.

**Decision:**
Never store trade state directly. Instead, derive state by replaying the witness chain - an append-only sequence of immutable action records.

**Rationale:**
- **Single Source of Truth**: The witness chain is the only source of truth; state is just a computed view
- **Eliminates State Bugs**: No possibility of state getting out of sync with history
- **Simplified Testing**: State derivation is a pure function with no I/O dependencies
- **Event Sourcing**: Natural event-sourced architecture without additional complexity

**Trade-Offs:**
- Positive: Eliminates entire class of state synchronization bugs, simplified testing, natural audit trail
- Negative: State computation overhead on every read (mitigated by being a simple linear scan)
- Slight performance cost for correctness guarantees

---

## Append-Only Witness Chain

**Context:**
Traditional systems allow modification or deletion of workflow actions. While this provides flexibility, it compromises audit integrity and makes it difficult to answer "what happened and when?"

**Decision:**
Implement witness chain as strictly append-only. Witnesses can never be modified or deleted once added.

**Rationale:**
- **Audit Integrity**: Complete, tamper-evident history of all actions
- **Simplicity**: No delete or update logic needed in the storage layer
- **Compliance**: Meets regulatory requirements for immutable audit trails
- **Debugging**: Full history available for troubleshooting issues

**Trade-Offs:**
- Positive: Regulatory compliance, simplified storage layer, complete forensic capabilities
- Negative: Cannot "undo" mistakes - must add corrective witnesses (e.g., Cancel)
- Operational flexibility for audit integrity

---

## CBOR Serialization

**Context:**
Need efficient, deterministic serialization for content addressing. Options considered:
- JSON: Human-readable but verbose, whitespace variations affect hashing
- Protocol Buffers: Efficient but requires schema management
- CBOR: Binary format with compact encoding

**Decision:**
Use CBOR (Concise Binary Object Representation) for all serialization needs.

**Rationale:**
- **Compact**: Binary encoding reduces storage and network overhead
- **Type-Safe**: Preserves type information without JSON's ambiguities
- **Rust Ecosystem**: Well-supported via `minicbor` crate
- **Hash-Stable**: Deterministic encoding suitable for content addressing
- **Standardized**: IETF RFC 8949 specification

**Consequences:**
- Positive: Efficient storage, deterministic hashing, good Rust support
- Negative: Not human-readable (requires tooling to inspect), non-canonical encoding possible
- Note: Canonical CBOR not enforced – standard CBOR encoding used by default

---

## Sled Embedded Database

**Context:**
Need persistent storage without complex database setup. Options considered:
- HashMap: In-memory only, no persistence
- SQLite: SQL overhead for key-value operations
- ReDB: Performance-oriented but had a complicated interface
- Sled: Pure Rust embedded key-value store

**Decision:**
Use Sled as the embedded database for content-addressable storage.

**Rationale:**
- **Native Rust**: No FFI boundaries, memory-safe by construction
- **Simple Interface**: Key-value operations match content-addressable model naturally
- **Transactional**: Batch operations for atomic commits
- **Low Latency**: Embedded design eliminates network overhead
- **Exploration**: Opportunity to evaluate Sled for future projects

**Consequences:**
- Positive: Simple API, native Rust, transactional guarantees, good performance
- Negative: Single-node only (not distributed), relatively new/less mature than alternatives
- Simplicity and Rust integration over distributed capabilities (not needed for this use case)

---

## Bech32 Encoded UUIDs

**Context:**
Need unique identifiers for trades, users, and other entities. Requirements:
- Global uniqueness
- Human-distinguishable (readable but not guessable)
- Error detection for typos
- Sortable by creation time

**Decision:**
Use UUID v7 (time-ordered) encoded with Bech32 format with human-readable prefixes (`trade_`, `user_`).

**Rationale:**
- **UUID v7**: Time-ordered for natural sorting, better DB indexing, globally unique
- **Bech32**: 6-byte checksum catches typos early, reducing operational errors
- **HRP Prefixes**: Human-readable prefixes (`trade_`, `user_`) make IDs self-documenting
- **Early Error Detection**: Checksum verification fails fast on malformed IDs

**Consequences:**
- Positive: Typo detection, self-documenting IDs, time-sortable, globally unique
- Negative: Longer string representation than raw UUIDs, requires encoding/decoding
- String length for error detection and readability

---

## Pure Functional Core Pattern

**Context:**
Service layer typically mixes business logic with I/O operations, making testing difficult and reasoning about correctness complex.

**Decision:**
Separate pure validation/state-derivation logic from I/O-bound service operations:
- `TradeContext::current_state()`: Pure function deriving state from witness chain
- `TradeContext::requires_approval()`: Pure function checking approval status
- `TradeService`: Stateless coordinator handling I/O and witness append

**Rationale:**
- **Testability**: Pure functions testable exhaustively without mocks or databases
- **Property Testing**: State derivation logic suitable for property-based testing
- **Clarity**: Clear separation between "what to do" (pure) and "how to persist" (I/O)
- **Reusability**: State derivation logic reusable in different contexts (CLI, API, batch jobs)

**Consequences:**
- Positive: Comprehensive testing possible, simplified reasoning, reusable logic separating pure from effectful code
- Negative: Requires discipline to maintain separation, slightly more code organisation
- Architectural clarity for minor complexity in code organisation

## Summary

The architecture embraces functional programming principles with content-addressable storage, treating all data as immutable facts. The witness chain provides an append-only event history from which state is derived, ensuring complete auditability and eliminating state synchronisation bugs. This design prioritises correctness, auditability, and testability over raw performance, making it well-suited for financial workflows where compliance and correctness are paramount.

Key themes:
- **Immutability First**: No mutation, only append operations
- **Derived State**: State computed from history, never stored
- **Audit by Design**: Complete history preserved automatically
- **Functional Core**: Pure logic separated from I/O effects
- **Type Safety**: Rust's type system prevents invalid states
