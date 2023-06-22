A dummy implementation of blockchain in Rust🦀

---

## Usage

### Run a node:

```bash
cargo run --bin node -- -n 1729
```

### Send a transaction:

```bash
cargo run --bin client -- -p 1729 -a <node address> txn <sender> <receiver> <value>  
```

---

## Limitations

- Currently, the blockchain does not maintain account balances.
- Either node or client don't need keypair to send messages or transactions.
- No auction model is implemented as making transactions don't need fee in this simulated blockchain.
- No specialised serialization is used for sending transactions / messages as can be seen with Ethereum using [RLP](https://ethereum.org/en/developers/docs/data-structures-and-encoding/rlp/) to serialize messages. Just a simple [binary serialization](https://docs.rs/bincode/latest/bincode/) is used. It is quite efficient though.

---