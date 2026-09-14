# Testing

## Rust tests

```bash
cargo test --workspace
```

This runs unit tests for:
- Protocol message serialization roundtrips (`crates/protocol`)
- Sensitive header masking (`apps/server`)
- Hop-by-hop header list completeness (`apps/server`)
- Password hashing and verification (`apps/server`)
- Target validation and forbidden schemes (`apps/agent`)

## Linting

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

## Dashboard builds

```bash
cd dashboards/admin && npx tsc --noEmit -p tsconfig.app.json && npm run build
cd dashboards/agent && npx tsc --noEmit -p tsconfig.app.json && npm run build
```

## End-to-end test

See [docs/development/setup.md](setup.md) for running the server and agent locally. The end-to-end flow:

1. Start the server.
2. Create a project and endpoint.
3. Send a webhook to the endpoint URL.
4. Verify the event appears in the dashboard.
5. Create an agent and get its token.
6. Log in the agent CLI.
7. Configure a local target.
8. Start the agent.
9. Replay the event from the dashboard.
10. Verify the local target receives the webhook.
11. Verify the delivery result is recorded locally.
