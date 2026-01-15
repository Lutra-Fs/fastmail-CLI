# Repository Guidelines

## Project Structure & Module Organization

- `fastmail-cli/` provides the CLI binary entry point and output formatting (`fastmail-cli/src/main.rs`).
- `fastmail-client/` encapsulates Fastmail-specific API logic and configuration.
- `jmap-client/` provides the generic JMAP HTTP client and shared types.
- `docs/` holds planning notes; `ref-docs/` stores RFCs and Fastmail references.

## Build, Test, and Development Commands

```bash
cargo build
```
Builds all workspace crates.

```bash
cargo test
```
Runs unit tests across the workspace.

```bash
cargo run -p fastmail-cli -- mail list --limit 10
```
Runs the CLI locally with arguments.

```bash
cargo install --path .
```
Installs the CLI from the workspace root.

## Coding Style & Naming Conventions

- Use `cargo fmt` for standard Rust formatting and readability.
- Keep crate boundaries aligned with responsibilities: CLI in `fastmail-cli`, API logic in `fastmail-client`, protocol and HTTP in `jmap-client`.
- Naming follows Rust conventions: `snake_case` for functions/vars, `CamelCase` for types.

## Testing Guidelines

- Tests use Rust’s built-in test framework and live alongside code (for example, `fastmail-client/src/client.rs`).
- Prefer small, focused tests for request building, parsing, and safety checks.
- Run `cargo test` before opening a PR.

## Commit & Pull Request Guidelines

- Commit messages follow Conventional Commits (for example, `feat(cli): ...`, `docs: ...`, `chore: ...`).
- PRs should include a clear description, rationale, and any new CLI examples or safety flags affected.
- If behavior changes, note expected JSON output or exit-code impact.

## Security & Configuration Tips

- Do not commit tokens; use the `FASTMAIL_TOKEN` environment variable.
- Keep safety flags such as `--force`, `--confirm`, and `--dry-run` consistent for delete or write paths.
