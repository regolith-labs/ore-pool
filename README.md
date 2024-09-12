# ORE Pool (beta)

**Infrastructure for ORE mining pools.**

## Admin
- Must `cargo run` the [admin application](./admin/src/main.rs) before starting server.
This creates the pool account on-chain which the server expects to exist upon starting.

## Considerations
- This implementation is still in active development and is subject to breaking changes.
- The idea is for this to be a reference implementation for operators.
- Feel free to fork this repo and add your custom logic.
- We're trying to add parameters of interest as [environment variables](./server/.env.example), but you can always fork if we've missed something in the meantime.
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
