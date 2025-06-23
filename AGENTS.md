# Contribution Guidelines

- Format all Rust code with `cargo fmt --all` before committing.
- Ensure the project compiles with `cargo check`.
- Place Rust modules in the `src/` directory, using one file per module for clarity.
- Add tests whenever you introduce new functionality. Make sure all test passed.
- Follow best practices for Rust code, project structure, documentation, logging, monitoring, etc.
- Follow best practices to implement the LSP server.
- Make sure the code work correctly with all php files in `examples/` folder.
- Bump the `version` field in `Cargo.toml` whenever new functionality is added.
- Update `docs/ROADMAP.md` whenever a milestone task is completed.
