# ORE Pool (beta)

**Infrastructure for operating ORE mining pools.**

## Admin
Must `cargo run` the [admin application](./admin/src/main.rs) before starting server.

1) Create the pool and member accounts on-chain which the server expects to exist upon starting. A member account is created because we need an account to write the pool commissions to.
You can manage this member account (stake, claim, etc.) from the `ore-cli`.
```sh
COMMAND="init" RPC_URL="" KEYPAIR_PATH="" POOL_URL="" cargo run --release
```
2) Create the stake account for each boost you want to support. Users can open share accounts that represent proportional shares in the total stake of the pool (per boost account).
```sh
COMMAND="open-stake" MINT="" RPC_URL="" KEYPAIR_PATH="" cargo run --release
```

## Server
There are many parameters that the server supports via [env vars](./server/.env.example). 
Including which boost accounts to support. How often to attribute members. And the webhook configuration.
```sh
RPC_URL="" KEYPAIR_PATH="" DB_URL="" ATTR_EPOCH="60" STAKE_EPOCH="60" BOOST_ONE="" HELIUS_API_KEY="" HELIUS_AUTH_TOKEN="" HELIUS_WEBHOOK_ID="" HELIUS_WEBHOOK_URL="http://your-server.com/webhook/share-account" OPERATOR_COMMISSION="" STAKER_COMMISSION="" RUST_LOG=info cargo run --release
```

## Webhook
The server depends on two [helius webhooks](https://docs.helius.dev/webhooks-and-websockets/what-are-webhooks).
1) One for tracking balance changes in the share/stake accounts. This is for proportionally attributing stakers in the pool.
2) The other is for tracking state changes to the proof account. This is for parsing the rewards (also for attribution).
- You'll need to create both webhooks manually in the helius dashboard. They should be of type `raw`.
- Also will need to generate an auth token that helius will include in their POST requests to your server. Pass this as an env var to the server.
- Creating new webhooks requires at least one address to listen for initially. For the share accounts webhook you can put any pubkey there initially,
the server will idempotently PUT to that list as new stakers join the pool (deleting the initial account you put there). For the proof account webhook, you want to put the proof account pubkey that belongs to the pool. You can find this pubkey by running the `proof-account` command in the [admin server](./admin/src/main.rs).
- Pass the webhook id for the share accounts to the server as an env var.
- One last detail is that testing on devnet the [webhook client](./server/src/webhook.rs) will set the RPC environment to mainnet. This isn't a problem in production. But if you happen to be testing in devnet, you'll need to manually keep an eye on that. We could fix this by including the RPC env in the PUT body. But we haven't seen that as a supported field, yet.


## Considerations
- This implementation is still in active development and is subject to breaking changes.
- The idea is for this to be a reference implementation for operators.
- Feel free to fork this repo and add your custom logic.
- Ofc we're accepting PRs / contributions. Please help us reach a solid v1.0.0.
- This implementation is integrated with the official `ore-cli`.
- So if you fork and change things, just make sure you serve the same HTTP paths that the `ore-cli` is interfacing with. If you do that, people should be able to participate in your pool with no additional installs or changes to their client.
- For reference, you'll find the required HTTP paths [here](./server/src/contributor.rs) and also the client-side API types [here](./types/src/lib.rs).

## For now
- For now this server only supports one "operator keypair" and thus one pool. So all of your members will participate in the same pool. This could be abstracted to support an arbitrary number of pools per operator server.
- For now this server does not collect commissions. We want to parameterize this as an env var.

## Local database
To spin up the database locally:
```
docker-compose up
```
