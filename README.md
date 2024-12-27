# ORE Pool (beta)

**Infrastructure for operating ORE mining pools.**

## Admin
Must `cargo run` the [admin application](./admin/src/main.rs) before starting server.

Creates the pool and member accounts on-chain which the server expects to exist upon starting. A member account is created because we need an account to write the pool commissions to.
You can manage this member account (balance, claim, etc.) from the `ore-cli`.
```sh
# cd ./admin
COMMAND="init" RPC_URL="" KEYPAIR_PATH="" POOL_URL="/my/path/id.json" cargo run --release
```

## Server
Start the server. Parameterized via [env vars](./server/.env.example).
```sh
# cd ./server
RPC_URL="" KEYPAIR_PATH="/my/path/id.json" DB_URL="" ATTR_EPOCH="60" HELIUS_AUTH_TOKEN="" OPERATOR_COMMISSION="" RUST_LOG=info cargo run --release
```

## Webhook
The server depends on a [helius webhook](https://docs.helius.dev/webhooks-and-websockets/what-are-webhooks),
for parsing the mining rewards asynchronously.
- You'll need to create the webhook manually in the helius dashboard. It should be of type `raw`.
- Also will need to generate an auth token that helius will include in their POST requests to your server. Pass this as an env var to the server.
- Creating new webhooks requires specifying the account address(es) to listen for. You want to put the proof account pubkey that belongs to the pool. You can find this pubkey by running the `proof-account` command in the [admin server](./admin/src/main.rs).

## Considerations
- This implementation is still in active development and is subject to breaking changes.
- The idea is for this to be a reference implementation for operators.
- Feel free to fork this repo and add your custom logic.
- Ofc we're accepting PRs / contributions. Please help us reach a solid v1.0.0.
- This implementation is integrated with the official `ore-cli`.
- If you fork and change things, just make sure you serve the same HTTP paths that the `ore-cli` is interfacing with. If you do that, people should be able to participate in your pool with no additional installs or changes to their client.
- For reference, you'll find the required HTTP paths [here](./server/src/contributor.rs) and also the client-side API types [here](./types/src/lib.rs).

## For now
- For now this server only supports one "operator keypair" and thus one pool. So all of your members will participate in the same pool. This could be abstracted to support an arbitrary number of pools per operator server.

## Local database
To spin up the database locally:
```
docker-compose up
```
