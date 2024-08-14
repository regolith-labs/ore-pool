# ORE Pool

**Infrastructure to manage an ORE mining pool.**


## API
- [`Consts`](api/src/consts.rs) – Program constants.
- [`Error`](api/src/error.rs) – Custom program errors.
- [`Event`](api/src/error.rs) – Custom program events.
- [`Instruction`](api/src/instruction.rs) – Declared instructions and arguments.

## Instructions
- [`Initialize`](program/src/initialize.rs) – Initializes the program and creates the global accounts.

## State
 - [`Pool`](api/src/state/pool.rs) – A singleton account ...


## Server

The server is for pool operators. 

To spin up the database locally:
```
docker-compose up
```

## Tests

To run the test suite, use the Solana toolchain: 

```
cargo test-sbf
```

For line coverage, use llvm-cov:

```
cargo llvm-cov
```
