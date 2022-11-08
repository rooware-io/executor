# Executor

Bare-bones Solana transaction playground, based off of [Neodyme's poc-framework](https://github.com/neodyme-labs/solana-poc-framework).

### Features

- Execute a transaction or a batch of transactions locally, and retrieve intermediate states (convenient to simulate e.g. Jito bundles execution)
- Automatically load all accounts involved in the transaction(s) from the specified cluster
- Consumable as:
  - Rust crate
  - HTTP server wrapper & client to decouple version sets, to avoid unnecessary dependency hell when possible

### Todo

- Fetch account through HTTP interface

### Gotchas

- This does not spin a local validator, and instead simply exposes a `Bank` to execute transactions and interact with accounts.
- Depending on your needs, you might be better off using (in increasing order of abstraction):
  - [Neodyme's poc-framework](https://github.com/neodyme-labs/solana-poc-framework)
  - solana's ProgramTest utility
  - solana-test-validator
  - RPC simulation
  - devnet
  - PROD!

### Workspace breakdown

- `core/`: transaction execution logic
- `server/`: wrapper server exposing the `core` logic through HTTP
- `client/`: client for above server
- `client-gen/`: library exposing a single macro to generate a client within your own crate, and avoid binding the solana dependencies (useful in case of dependency pinning hell, even though versions are compatible)
- `example/`: sample usage of the client-gen
