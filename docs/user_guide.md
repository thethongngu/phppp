# phppp User Guide

This guide covers basic usage of the phppp language server.

## Installation

Download a release from the GitHub releases page or build from source using:

```bash
cargo build --release
```

Copy the resulting `phppp` binary somewhere in your PATH.

## Using with VS Code

Install the phppp extension from the Marketplace. The server starts automatically when you open a PHP workspace.

## Configuration

Create a `.phppprc` file in your project root to enable optional features:

```json
{ "enable_laravel": true }
```

See the README for full configuration details.

## Troubleshooting

Run the server with `RUST_LOG=debug` to see verbose logs if you encounter problems. Check the extension output panel for errors.

