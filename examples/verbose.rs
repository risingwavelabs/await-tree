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

//! This example shows how to mark a span as "verbose", so that it's conditionally
//! enabled based on the config.

use std::time::Duration;

use await_tree::{ConfigBuilder, InstrumentAwait, Registry, SpanExt};
use futures::future::pending;
use tokio::time::sleep;

async fn foo() {
    // verbose span will be disabled if the `verbose` flag in the config is false
    pending().instrument_await("pending".verbose()).await
}

async fn work(verbose: bool) -> String {
    let config = ConfigBuilder::default().verbose(verbose).build().unwrap();
    let registry = Registry::new(config);
    let root = registry.register((), "foo");
    tokio::spawn(root.instrument(foo()));

    sleep(Duration::from_secs(1)).await;
    registry.get(()).unwrap().to_string()
}

#[tokio::main]
async fn main() {
    // foo [1.001s]
    println!("{}", work(false).await);

    // foo [1.004s]
    //   pending [1.004s]
    println!("{}", work(true).await);
}
