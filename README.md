# phppp
A Rust-based PHP LSP.

## Building

To build the project:

```bash
cargo build
```

To build in release mode:

```bash
cargo build --release
```

## Running Tests

To run all tests:

```bash
cargo test
```

To run tests with output:

```bash
cargo test -- --nocapture
```

## Example

Run the example parser on a PHP file:

```bash
cargo run --bin example examples/hello.php
```

## Architecture

The server is built using **tree-sitter** for parsing PHP source files. Parsed
syntax trees are fed into an indexer that collects functions, classes, constants
and variables for quick lookup. An analyzer then resolves symbol definitions
across documents, while the LSP layer powered by `tower-lsp` exposes completion,
hover and go-to-definition features.

## Running the LSP Server

To run the main LSP server:

```bash
cargo run
```

The server logs messages to the editor's **Output** panel. You can also restart
the server from your editor by executing the `phppp.restart` command.

## Configuration

phppp reads a `.phppprc` file from your workspace root. Currently the file is
JSON formatted and supports the following option:

- `enable_laravel` - when set to `true`, registers additional helpers for
  Laravel projects.

Example `.phppprc`:

```json
{ "enable_laravel": true }
```
