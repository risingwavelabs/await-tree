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

//! This example shows how to use `await-tree` for multiple actors.

use std::time::Duration;

use await_tree::{span, Config, InstrumentAwait, Registry};
use futures::future::pending;
use itertools::Itertools;
use tokio::time::sleep;

async fn work(i: i32) {
    foo().instrument_await(span!("actor work {i}")).await
}

async fn foo() {
    pending().instrument_await("pending").await
}

#[tokio::main]
async fn main() {
    let registry = Registry::new(Config::default());
    for i in 0_i32..3 {
        let root = registry.register(i, format!("actor {i}"));
        tokio::spawn(root.instrument(work(i)));
    }

    sleep(Duration::from_secs(1)).await;

    // actor 0 [1.007s]
    //   actor work 0 [1.007s]
    //     pending [1.007s]
    //
    // actor 1 [1.007s]
    //   actor work 1 [1.007s]
    //     pending [1.007s]
    //
    // actor 2 [1.007s]
    //   actor work 2 [1.007s]
    //     pending [1.007s]
    for (_, tree) in registry
        .collect::<i32>()
        .into_iter()
        .sorted_by_key(|(i, _)| *i)
    {
        println!("{tree}");
    }
}
