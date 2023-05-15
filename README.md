# await-tree

[![Crate](https://img.shields.io/crates/v/await-tree.svg)](https://crates.io/crates/await-tree)
[![Docs](https://docs.rs/await-tree/badge.svg)](https://docs.rs/await-tree)

The `Future`s in Async Rust can be arbitrarily composited or nested to achieve a variety of control flows.
Assuming that the execution of each `Future` is represented as a node,
then the asynchronous execution of an async task can be organized into a **logical tree**,
which is constantly transformed over the polling, completion, and cancellation of `Future`s.

`await-tree` allows developers to dump this execution tree at runtime, with the span of each `Future` annotated by `instrument_await`. A basic example is shown below, and more examples of complex control flows can be found in the [examples](./examples) directory.

```rust
async fn bar(i: i32) {
    // `&'static str` span
    baz(i).instrument_await("baz in bar").await
}

async fn baz(i: i32) {
    // runtime `String` span is also supported
    pending()
        .instrument_await(format!("pending in baz {i}"))
        .await
}

async fn foo() {
    // spans of joined futures will be siblings in the tree
    join(
        bar(3).instrument_await("bar"),
        baz(2).instrument_await("baz"),
    )
    .await;
}

let root = register("foo");
tokio::spawn(root.instrument(foo()));

sleep(Duration::from_secs(1)).await;
let tree = get_tree("foo");

// foo [1.006s]
//   bar [1.006s]
//     baz in bar [1.006s]
//       pending in baz 3 [1.006s]
//   baz [1.006s]
//     pending in baz 2 [1.006s]
println!("{tree}");
```

### Compared to `async-backtrace`

[`tokio-rs/async-backtrace`](https://github.com/tokio-rs/async-backtrace) is a similar crate that also provides the ability to dump the execution tree of async tasks. Here are some differences between `await-tree` and `async-backtrace`:

**Pros of `await-tree`**:
- `await-tree` support customizing the span with runtime `String`, while `async-backtrace` only supports function name and line number.

  This is useful when we want to annotate the span with some dynamic information, such as the identifier of a shared resource (e.g., a lock), to see how the contention happens among different tasks.

- `await-tree` support almost all kinds of async control flows with arbitrary `Future` topology, while `async-backtrace` fails to handle some of them.

  For example, it's common to use `&mut impl Future` as an arm of `select` to avoid problems led by cancellation unsafety. To further resolve this `Future` after the `select` completes, we may move it to another place and `await` it there. `async-backtrace` fails to track this `Future` again due to the change of its parent. See [`examples/detach.rs`](./examples/detach.rs) for more details.

- `await-tree` maintains the tree structure with an [arena-based data structure](https://crates.io/crates/indextree), with zero extra `unsafe` code. For comparison, `async-backtrace` crafts it by hand and there's potential memory unsafety for unhandled topologies mentioned above.

  It's worth pointing out that `await-tree` has been applied in the production deployment of [RisingWave](https://github.com/risingwavelabs/risingwave), a distributed streaming database, for a long time.

- `await-tree` maintains the tree structure separately from the `Future` itself, which enables developers to dump the tree at any time with nearly no contention, no matter the `Future` is under active polling or has been pending. For comparison, `async-backtrace` has to [wait](https://docs.rs/async-backtrace/0.2.5/async_backtrace/fn.taskdump_tree.html) for the polling to complete before dumping the tree, which may cause a long delay.

**Pros of `async-backtrace`**:
- `async-backtrace` is under the Tokio organization.

## License

`await-tree` is distributed under the Apache License (Version 2.0). Please refer to [LICENSE](./LICENSE) for more information.
