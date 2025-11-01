# Devlog

Build a trade approval system. Users submit potiential trade details for approval and go through a structured workflow of validation logic before execution or cancelled. We need to streamline the processess and standardise the approval process for different finacial instruments such as 'forward contacts'

The following actions are defined:
- Submit: A trade is thrown into the current pool awaiting approval
- Update: change the details of the trade, though doing this sends it back to queue awaiting approval
- Cancel: At any point before execution the trade can be canceled and droped.
- Accept: Submitted requests are moved into the execution stage.
- Execute: A finialised trade is sent to the counter-party to await completion
- Book: Successfully executed trades are appended into a ledger making an account of what happend.

the trade details require the following fields:

```
// going to call it a transaction instead of a trade makes more sense to me that way

Transaction {
    tradingEntity // who intialises the transaction
    counterParty// who are we trading with?
    direction // Either Buy or Sell
    style // assumes its a Forward Contract
    notionalCurrency // should include ticker and IBAN Code
    notionalAmmount // size of the trade in the selected currency
    uderlying // a combination of other notional currencies. (is this the other currency where trading with?)
    trade_date // marks the date the trade was submitted
    value_date // marks the date the was executed
    delivery_date // markst the date the assets were delivered
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
4. Approved: Trades approved can now be send to the counter party
5. SentToCounterParty: The trade has now been sent to the counter party
6. Executed: if the trade was sucessfully executed, book it
7. Cancelled: trades can be canceled at any time before execution.


### Notes
- Could a session type be used here?
- The last thing we want is to have methods available on transactions that shouldnt be there depending on the state of the transaction. strongly typing the relationship give us gurantess on what stage the Transaction is at enforcing type checks at each stage. an improperly typed trade fails.
    - Ordering is important here, such that a draft trade cannot be sent to the counter party or be confused for a submitted trade for example.

## Architecture
- transactions are:
    - immutable: every transaction or trade (from now on we will call it a transaction to avoid confusion) is immutable. each state transition is esstionally a copy-on- write so state and built upon a series of events creating a history for replaying events.
    - state: state is ephemeral here such that we formalise our transactions ahead of time keeping context local to the actions that we perform. state is derived from its predessesor as mentioned above we going with a copy-on-write approach.
    - validation: a predicate who's only job is to tell us "is this transaction valid to execute right now?"

The key benifits here is that we can catch erros and formalise "correct" transactions before weve even submitted them to a network. this way everything is offline-local, no databases are needed until we have a completed "correct" trade.

There are two components to this lib. One is the validator logic the other on e is our service api layer.

## Validation
- did the right user approve this trade? are the dates correct? was it updated then re-approved? etc;
- witnesses are checkd as part of the pending transactions to check what stage it's at. we can use this show the currency status to the user.

## Why CBOR?
CBOR is a small enough binary encoding that should we wish to send trades across machines over a network effciently assuming the boundries are uniform, we can. Plus if trades are going to be determined by a hash then it requres that the sequence of bytes remains in tact if I were to use JSON some of the types plus whitespace and lack of number precision, with json being a native base64 encoding size is inflated adding extra bytes to compensate which could potentially change the hash data integrity is lost when using JSON; Espcially if were dealing with content addressable stores and hashes.
