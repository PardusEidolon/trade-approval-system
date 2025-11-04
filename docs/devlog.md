# Devlog

*Notes I made to help me understand the problem.*

Build a trade approval system. Users submit potential trade details for approval and go through a structured workflow of validation logic before execution or cancellation. We need to streamline the processes and standardise the approval process for different financial instruments such as 'forward contracts'.

The following actions are defined:
- Submit: A trade is thrown into the current pool awaiting approval
- Update: Change the details of the trade, though doing this sends it back to queue awaiting approval
- Cancel: At any point before execution the trade can be cancelled and dropped
- Accept: Submitted requests are moved into the execution stage
- Execute: A finalised trade is sent to the counter-party to await completion
- Book: Successfully executed trades are appended into a ledger making an account of what happened

the trade details require the following fields:

```
// going to call it a transaction instead of a trade makes more sense to me that way

Transaction {
    tradingEntity // who initialises the transaction
    counterParty // who are we trading with?
    direction // Either Buy or Sell
    style // assumes it's a Forward Contract
    notionalCurrency // should include ticker and IBAN Code
    notionalAmount // size of the trade in the selected currency
    underlying // a combination of other notional currencies. (is this the other currency we're trading with?)
    trade_date // marks the date the trade was submitted
    value_date // marks the date the trade was executed
    delivery_date // marks the date the assets were delivered
    strike // the agreed upon rate after the trade was executed
}
```

The following rule must be met before execution!
```
trade_date <= value_date <= delivery_date
```

## Transitions
Explain the flow of a transaction

1. Draft: Trades are drafted before submission
2. PendingApproval: Submitted trades await approval
3. NeedsReApproval: Trades that were updated need to be re-approved again
4. Approved: Trades approved can now be sent to the counter party
5. SentToCounterParty: The trade has now been sent to the counter party
6. Executed: If the trade was successfully executed, book it
7. Cancelled: Trades can be cancelled at any time before execution


### Notes
- Could a session type be used here?
- The last thing we want is to have methods available on transactions that shouldn't be there depending on the state of the transaction. Strongly typing the relationship gives us guarantees on what stage the transaction is at, enforcing type checks at each stage. An improperly typed trade fails.
    - Ordering is important here, such that a draft trade cannot be sent to the counter party or be confused for a submitted trade, for example.

## Architecture
- transactions are:
    - immutable: Every transaction or trade (from now on we will call it a transaction to avoid confusion) is immutable. Each state transition is essentially a copy-on-write, so state is built upon a series of events creating a history for replaying events.
    - state: State is ephemeral here such that we formalise our transactions ahead of time, keeping context local to the actions that we perform. State is derived from its predecessor; as mentioned above, we're going with a copy-on-write approach.
    - validation: A predicate whose only job is to tell us "is this transaction valid to execute right now?"

The key benefits here are that we can catch errors and formalise "correct" transactions before we've even submitted them to a network. This way everything is offline-local, no databases are needed until we have a completed "correct" trade.

There are two components to this lib. One is the validator logic; the other one is our service API layer.

## Validation
- Did the right user approve this trade? Are the dates correct? Was it updated then re-approved? etc.
- Witnesses are checked as part of the pending transactions to check what stage it's at. We can use this to show the current status to the user.

## Why CBOR?
Note: Canonicalised CBOR is not enforced here, so hashes could return malformed. Encoding is not strict and minicbor produces standard CBOR by default.

CBOR is a small enough binary encoding that is efficient and hash safe.

## Why Sled?
- Because I was looking around at different embedded database storage implementations for my own storage layer side project in Rust and Sled stood out for having a native Rust interface that wasn't SQL. I could have used a HashMap for the purposes of my design, but I wanted storage on disk, not memory, whilst still having low latency I/O. Plus I wanted to try it out.

## A Note on IDs and Witnesses
Initially you want public/private keys to perform signatures on the serialised trades before we insert and append them into our trade context. However, to get the idea across I'll just use bech32-encoded UUID strings. So when we approve a trade, for example, we can treat this as a kind of signature.

### Why bech32?
Because it uses a 6-byte checksum which brings down the cost of malformed IDs. Plus, IDs are copied across witness sets. If witnesses are going to refer to the current trade context, we need to make sure we're not putting different IDs. So if trade_ids are going to be unique, it would be nice to have early error detection when entering past trade IDs. But if in the case we change a byte in a UUID, then it's a completely different trade.

## ID Scheme

```
<human_readable_part> : <uuid_bech32_encoded_string>

// current choices
'trade_': trade identifier separated by an '_'
'user_': user identifier separated by an '_'
```
