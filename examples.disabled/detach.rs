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

//! This example shows how a span can be detached from and remounted to the tree.

use std::time::Duration;

use await_tree::{Config, InstrumentAwait, Registry};
use futures::channel::oneshot::{self, Receiver};
use futures::future::{pending, select};
use futures::FutureExt;
use tokio::time::sleep;

async fn work(rx: Receiver<()>) {
    let mut fut = pending().instrument_await("fut");

    // poll `fut` under the `select` span
    let _ = select(
        sleep(Duration::from_millis(500))
            .instrument_await("sleep")
            .boxed(),
        &mut fut,
    )
    .instrument_await("select")
    .await;

    // `select` span closed so `fut` is detached
    // the elapsed time of `fut` should be preserved

    // wait for the signal to continue
    rx.instrument_await("rx").await.unwrap();

    // poll `fut` under the root `work` span, and it'll be remounted
    fut.await
}

#[tokio::main]
async fn main() {
    let registry = Registry::new(Config::default());
    let root = registry.register((), "work");
    let (tx, rx) = oneshot::channel();
    tokio::spawn(root.instrument(work(rx)));

    sleep(Duration::from_millis(100)).await;
    let tree = registry.get(&()).unwrap().to_string();

    // work [106.290ms]
    //   select [106.093ms]
    //     sleep [106.093ms]
    //     fut [106.093ms]
    println!("{tree}");

    sleep(Duration::from_secs(1)).await;
    let tree = registry.get(&()).unwrap().to_string();

    // work [1.112s]
    //   rx [606.944ms]
    // [Detached 4]
    //   fut [1.112s]
    println!("{tree}");

    tx.send(()).unwrap();
    sleep(Duration::from_secs(1)).await;
    let tree = registry.get(&()).unwrap().to_string();

    // work [2.117s]
    //   fut [2.117s]
    println!("{tree}");
}
