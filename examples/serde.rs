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

//! This example shows the serialization format of the tree.
//!
//! The execution flow is the same as `examples/detach.rs`.

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
    let tree = serde_json::to_string_pretty(&registry.get(()).unwrap()).unwrap();

    // {
    //   "current": 1,
    //   "tree": {
    //     "id": 1,
    //     "span": {
    //       "name": "work",
    //       "is_verbose": false,
    //       "is_long_running": true
    //     },
    //     "elapsed_ns": 105404875,
    //     "children": [
    //       {
    //         "id": 2,
    //         "span": {
    //           "name": "select",
    //           "is_verbose": false,
    //           "is_long_running": false
    //         },
    //         "elapsed_ns": 105287624,
    //         "children": [
    //           {
    //             "id": 3,
    //             "span": {
    //               "name": "sleep",
    //               "is_verbose": false,
    //               "is_long_running": false
    //             },
    //             "elapsed_ns": 105267874,
    //             "children": []
    //           },
    //           {
    //             "id": 4,
    //             "span": {
    //               "name": "fut",
    //               "is_verbose": false,
    //               "is_long_running": false
    //             },
    //             "elapsed_ns": 105264874,
    //             "children": []
    //           }
    //         ]
    //       }
    //     ]
    //   },
    //   "detached": []
    // }
    println!("{tree}");

    sleep(Duration::from_secs(1)).await;
    let tree = serde_json::to_string_pretty(&registry.get(()).unwrap()).unwrap();

    // {
    //   "current": 1,
    //   "tree": {
    //     "id": 1,
    //     "span": {
    //       "name": "work",
    //       "is_verbose": false,
    //       "is_long_running": true
    //     },
    //     "elapsed_ns": 1108552791,
    //     "children": [
    //       {
    //         "id": 3,
    //         "span": {
    //           "name": "rx",
    //           "is_verbose": false,
    //           "is_long_running": false
    //         },
    //         "elapsed_ns": 603081749,
    //         "children": []
    //       }
    //     ]
    //   },
    //   "detached": [
    //     {
    //       "id": 4,
    //       "span": {
    //         "name": "fut",
    //         "is_verbose": false,
    //         "is_long_running": false
    //       },
    //       "elapsed_ns": 1108412791,
    //       "children": []
    //     }
    //   ]
    // }
    println!("{tree}");

    tx.send(()).unwrap();
    sleep(Duration::from_secs(1)).await;
    let tree = serde_json::to_string_pretty(&registry.get(()).unwrap()).unwrap();

    // {
    //   "current": 1,
    //   "tree": {
    //     "id": 1,
    //     "span": {
    //       "name": "work",
    //       "is_verbose": false,
    //       "is_long_running": true
    //     },
    //     "elapsed_ns": 2114497458,
    //     "children": [
    //       {
    //         "id": 4,
    //         "span": {
    //           "name": "fut",
    //           "is_verbose": false,
    //           "is_long_running": false
    //         },
    //         "elapsed_ns": 2114366458,
    //         "children": []
    //       }
    //     ]
    //   },
    //   "detached": []
    // }
    println!("{tree}");
}
