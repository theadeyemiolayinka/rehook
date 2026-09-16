# Contributing to Rehook

Thank you for your interest in contributing to Rehook. This document covers the basics of getting set up and submitting changes.

## Development setup

See [docs/development/setup.md](docs/development/setup.md) for instructions on building and running Rehook from source.

## Before submitting a pull request

1. Run `cargo fmt --all -- --check` and fix any formatting issues.
2. Run `cargo clippy --workspace --all-targets` and fix any warnings.
3. Run `cargo test --workspace` and ensure all tests pass.
4. If you changed the admin dashboard, run `cd dashboards/admin && npm run build`.
5. If you changed the agent dashboard, run `cd dashboards/agent && npm run build`.
6. Write clear commit messages. Focus on why, not what.

## Security

If you find a security vulnerability, do not open a public issue. See [SECURITY.md](SECURITY.md) for reporting instructions.

## Code style

- Follow idiomatic Rust. Prefer clear names over comments.
- Do not add comments that state what the code obviously does.
- Do add comments for security reasoning, protocol behavior, and non-obvious decisions.
- Do not introduce new dependencies without justification.
- Do not weaken the SSRF or target validation model. See [docs/architecture/security-model.md](docs/architecture/security-model.md).

## License

By contributing, you agree that your contributions will be licensed under the same license as the project. The license is specified in the workspace `Cargo.toml`.
