// Copyright 2025 RisingWave Labs
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

//! This example shows how to mark a span as "long_running", so that it will
//! not be marked as "!!!" if it takes too long to complete.

use std::time::Duration;

use await_tree::{Config, InstrumentAwait, Registry, SpanExt};
use futures::future::{pending, select};
use futures::FutureExt;
use tokio::time::sleep;

async fn long_running_child() {
    pending()
        .instrument_await("long_running_child".long_running())
        .await
}

async fn child() {
    pending().instrument_await("child").await
}

async fn foo() {
    select(long_running_child().boxed(), child().boxed()).await;
}

async fn work() -> String {
    let registry = Registry::new(Config::default());
    let root = registry.register((), "foo");
    tokio::spawn(root.instrument(foo()));

    // The default threshold is 10 seconds.
    sleep(Duration::from_secs(11)).await;
    registry.get(()).unwrap().to_string()
}

#[tokio::main]
async fn main() {
    // foo [11.006s]
    //   long_running_child [11.006s]
    //   child [!!! 11.006s]
    println!("{}", work().await);
}
