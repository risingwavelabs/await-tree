// Copyright 2023 RisingWave Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! This example shows how to spawn tasks with `await_tree::spawn` that are automatically registered
//! to the current registry of the scope.
//!
//! Note: This example requires the `tokio` feature to be enabled.
//! Run with: `cargo run --example spawn --features tokio`

#![cfg(feature = "tokio")]

use std::time::Duration;

use await_tree::{Config, InstrumentAwait, Registry};
use futures::future::pending;
use tokio::time::sleep;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Actor(usize);

async fn actor(i: usize) {
    // Since we're already inside the scope of a registered/instrumented task, we can directly spawn
    // new tasks with `await_tree::spawn` to also register them in the same registry.
    await_tree::spawn_anonymous(format!("background task {i}"), async {
        pending::<()>().await;
    })
    .instrument_await("waiting for background task")
    .await
    .unwrap();
}

#[tokio::main]
async fn main() {
    let registry = Registry::new(Config::default());

    for i in 0..3 {
        let root = registry.register(Actor(i), format!("actor {i}"));
        tokio::spawn(root.instrument(actor(i)));
    }

    sleep(Duration::from_secs(1)).await;

    for (_actor, tree) in registry.collect::<Actor>() {
        // actor 0 [1.004s]
        //   waiting for background task [1.004s]
        println!("{tree}");
    }
    for tree in registry.collect_anonymous() {
        // background task 0 [1.004s]
        println!("{tree}");
    }
}
