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

//! Example demonstrating the use of the `#[await_tree::instrument]` attribute macro.

use await_tree::{init_global_registry, instrument, spawn_derived_root, ConfigBuilder, Registry};
use std::time::Duration;
use tokio::time::sleep;

#[instrument(long_running, "fetch_data({})", id)]
async fn fetch_data(id: u32) -> String {
    sleep(Duration::from_millis(100)).await;
    format!("data_{}", id)
}

#[instrument(verbose, "process_item({}, {})", name, value)]
async fn process_item(name: &str, value: i32) -> i32 {
    sleep(Duration::from_millis(50)).await;
    value * 2
}

#[instrument(long_running, verbose, "complex_operation")]
async fn complex_operation() -> Vec<String> {
    let mut results = Vec::new();

    for i in 1..=3 {
        let data = fetch_data(i).await;
        results.push(data);
    }

    let processed = process_item("test", 42).await;
    results.push(format!("processed: {}", processed));

    results
}

#[instrument]
async fn simple_task() -> String {
    sleep(Duration::from_millis(100)).await;
    "simple result".to_string()
}

#[tokio::main]
async fn main() {
    // Initialize the global registry
    init_global_registry(ConfigBuilder::default().verbose(true).build().unwrap());

    // Spawn tasks with instrumentation
    spawn_derived_root("complex", complex_operation());
    spawn_derived_root("simple", simple_task());

    // Let the tasks run for a while
    sleep(Duration::from_millis(50)).await;

    // Print the await trees
    if let Some(tree) = Registry::current().get("complex") {
        println!("Complex task tree:");
        println!("{}", tree);
        println!();
    }

    if let Some(tree) = Registry::current().get("simple") {
        println!("Simple task tree:");
        println!("{}", tree);
    }
}
