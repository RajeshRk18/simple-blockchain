## This is a very simple and dummy implementation of a blockchain that I did to learn Rust🦀🦀

---

## To spin up one:

```bash

    git clone https://github.com/RajeshRk18/simple-blockchain.git 

    cd simple-blockchain

    cargo run --release
```

---

## Note🚨

### This blockchain can mine only upto 64 blocks as I have set each byte as bit to make it simple. You can see that below.


src/blockchain.rs | line: 37
``` Rust
hash_gen.split_at(difficulty).0.as_bytes()[0..difficulty]
```

---