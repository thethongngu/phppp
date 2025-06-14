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
