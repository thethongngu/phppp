# phppp LSP Roadmap

This roadmap outlines major milestones to evolve **phppp** into the most popular PHP Language Server for VS Code. Each milestone lists key features and a TODO list to track progress.

## Milestone 1: Stable Core

Provide a solid foundation with a reliable parser, analyzer and LSP implementation.

### Features
- Robust parsing of PHP 8+ syntax
- Comprehensive AST and symbol table management
- Accurate diagnostics (syntax and semantic errors)
- Basic completion, hover and go-to-definition support

### TODO
- [x] Expand parser test coverage using examples in `examples/`
- [x] Integrate tree-sitter updates for latest PHP grammar
- [x] Document architecture in `README.md`
- [x] Ensure `cargo check` and `cargo test` pass in CI

## Milestone 2: Advanced IDE Features

Enhance developer productivity with advanced analysis and refactoring tools.

### Features
- Reference search and rename symbol
- Workspace-wide symbol index
- Code actions for common refactors
- Incremental diagnostics with real-time feedback

### TODO
- [x] Implement indexer module for workspace scanning
- [x] Add integration tests for rename and references
- [x] Support incremental file watching via `notify` crate
- [ ] Profile and optimize analyzer performance

## Milestone 3: Ecosystem Integration

Ensure the server works seamlessly in major PHP frameworks and tools.

### Features
- Composer dependency awareness
- Framework-specific helpers (Laravel, Symfony, etc.)
- Configuration file support (`.phppprc`, VS Code settings)
- Extension API for community plugins

### TODO
- [x] Resolve class paths from `composer.json`
- [x] Prototype Laravel helper plugin
- [x] Document configuration options
- [x] Publish VS Code extension on Marketplace

## Milestone 4: Reliability & Observability

Operate the LSP at production scale with robust logging and monitoring.

### Features
- Structured logging with configurable levels
- Metrics collection (response times, errors)
- Crash reporting and recovery
- Continuous benchmarking

### TODO
- [ ] Integrate `tracing` for structured logs
- [ ] Expose Prometheus metrics endpoint
- [ ] Add stress tests simulating large projects
- [ ] Automate release builds with GitHub Actions

## Milestone 5: Community & Adoption

Build a strong community to drive adoption and contributions.

### Features
- Comprehensive documentation and tutorials
- Issue templates and contributor guides
- Regular release cycle with changelogs
- Outreach via blog posts and social media

### TODO
- [ ] Expand `docs/` with user and dev guides
- [ ] Add GitHub templates for issues and PRs
- [ ] Set up monthly release schedule
- [ ] Engage with PHP community conferences

---

Reaching these milestones will position **phppp** as a full-featured, stable and widely adopted PHP LSP for VS Code.
