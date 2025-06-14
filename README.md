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

## Running the LSP Server

To run the main LSP server:

```bash
cargo run
```

The server logs messages to the editor's **Output** panel. You can also restart
the server from your editor by executing the `phppp.restart` command.
