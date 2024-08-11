# ORE Pool

**A program to manage ORE mining pools.**


## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Event`](api/src/error.rs) – Custom program events.
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments.

## Instructions
- [`Initialize`](program/src/initialize.rs) – Initializes the program and creates the global accounts.

## State
 - [`Pool`](api/src/state/pool.rs) – A singleton account ...


## Tests

To run the test suite, use the Solana toolchain: 

```
cargo test-sbf
```

For line coverage, use llvm-cov:

```
cargo llvm-cov
```
