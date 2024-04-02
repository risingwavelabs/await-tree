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

//! This example shows the usage of the global registry.

use std::time::Duration;

use await_tree::{init_global_registry, Config, InstrumentAwait, Registry};
use futures::future::pending;

async fn bar() {
    pending::<()>().instrument_await("pending").await;
}

async fn foo() {
    await_tree::spawn_anonymous("spawn bar", bar());
    bar().instrument_await("bar").await;
}

async fn print() {
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Access the registry anywhere and collect all trees.
    for (key, tree) in Registry::current().collect_all() {
        println!("[{}]\n{}\n", key, tree);
    }
}

#[tokio::main]
async fn main() {
    init_global_registry(Config::default());

    // After global registry is initialized, the tasks can be spawned everywhere, being
    // registered in the global registry.
    await_tree::spawn("Actor 42", "foo", foo());

    // The line above is a shorthand for the following:
    tokio::spawn(
        Registry::current()
            .register("Print", "print")
            .instrument(print()),
    )
    .await
    .unwrap();
}
