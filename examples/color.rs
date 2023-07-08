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

//! This example shows how to configure warn threshold and color the warn


use std::time::Duration;
use await_tree::{Config, InstrumentAwait, Registry};
use tokio::time::sleep;

async fn short_work(){
    sleep(Duration::from_millis(500)).instrument_await("short").await
}

async fn long_work(){
    sleep(Duration::from_millis(5000)).instrument_await("long").await
}

#[tokio::main]
async fn main() {
    let mut registry = Registry::new(Config{
        verbose: true,
        colored: true,
        warn_threshold: Duration::from_millis(1000).into(),
    });

    let root = registry.register((), "work");
    tokio::spawn(root.instrument(async {
        short_work().await;
        long_work().await;
    }));

    sleep(Duration::from_millis(100)).await;
    let tree = registry.get(&()).unwrap().to_string();

    // work
    //   short [105.606ms]
    println!("{tree}");


    sleep(Duration::from_millis(2000)).await;
    let tree = registry.get(&()).unwrap().to_string();

    // work
    //   long [1.609s]
    println!("{tree}");
}
