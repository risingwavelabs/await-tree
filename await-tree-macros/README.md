# await-tree-macros

Procedural macros for the [`await-tree`](https://crates.io/crates/await-tree) crate.

## Overview

This crate provides the `#[instrument]` attribute macro that automatically instruments async functions with await-tree spans, similar to how `tracing::instrument` works but specifically designed for await-tree.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
await-tree = { version = "0.3", features = ["macros"] }
```

Then use the `#[instrument]` attribute on your async functions:

```rust
use await_tree::{instrument, InstrumentAwait};

#[instrument("fetch_data({})", id)]
async fn fetch_data(id: u32) -> String {
    // Your async code here
    format!("data_{}", id)
}

#[instrument]
async fn simple_function() -> String {
    "hello".to_string()
}
```

## Macro Expansion

The `#[instrument]` macro transforms your async function by:

1. Creating an await-tree span with the provided format arguments
2. Wrapping the original function body in an async block
3. Instrumenting the async block with the span

For example:

```rust
#[instrument("span_name({})", arg1)]
async fn foo(arg1: i32, arg2: String) {
    // original function body
}
```

Expands to:

```rust
async fn foo(arg1: i32, arg2: String) {
    let span = await_tree::span!("span_name({})", arg1);
    let fut = async move {
        // original function body
    };
    fut.instrument_await(span).await
}
```

## Features

- **Format arguments**: Pass format strings and arguments just like `format!()` or `println!()`
- **No argument parsing**: Format arguments are passed directly to `await_tree::span!()` without modification
- **Function name fallback**: If no arguments are provided, uses the function name as the span name
- **Preserves function attributes**: All function attributes and visibility modifiers are preserved

## Requirements

- The macro can only be applied to `async` functions
- You must import `InstrumentAwait` trait to use the generated code
- The `macros` feature must be enabled in the `await-tree` dependency

## License

Licensed under the Apache License, Version 2.0.
