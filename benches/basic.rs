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

use std::time::Duration;

use await_tree::{Config, ConfigBuilder, InstrumentAwait, Registry};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::{Builder, Runtime};
use tokio::task::yield_now;

fn runtime() -> Runtime {
    Builder::new_current_thread().enable_time().build().unwrap()
}

async fn test() {
    async fn test_inner() {
        futures::future::join(
            async {
                yield_now().await;
                black_box(1)
            }
            .instrument_await("fut1"),
            async {
                yield_now().await;
                yield_now().await;
                black_box(2)
            }
            .instrument_await("fut2"),
        )
        .instrument_await("join")
        .await;
    }

    for _ in 0..10000 {
        test_inner().await;
    }
}

async fn test_baseline() {
    async fn test_inner() {
        futures::future::join(
            async {
                yield_now().await;
                black_box(1)
            },
            async {
                yield_now().await;
                yield_now().await;
                black_box(2)
            },
        )
        .await;
    }

    for _ in 0..10000 {
        test_inner().await;
    }
}

async fn spawn_many(size: usize) {
    let registry = Registry::new(Config::default());
    let mut handles = vec![];
    for i in 0..size {
        let task = async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        handles.push(tokio::spawn(
            registry.register(i, "new_task").instrument(task),
        ));
    }
    futures::future::try_join_all(handles)
        .await
        .expect("failed to join background task");
}

async fn spawn_many_baseline(size: usize) {
    let mut handles = vec![];
    for _ in 0..size {
        let task = async {
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        handles.push(tokio::spawn(task));
    }
    futures::future::try_join_all(handles)
        .await
        .expect("failed to join background task");
}

// time:   [6.5488 ms 6.5541 ms 6.5597 ms]
// change: [+6.5978% +6.7838% +6.9299%] (p = 0.00 < 0.05)
// Performance has regressed.
fn bench_basic(c: &mut Criterion) {
    c.bench_function("basic", |b| {
        b.to_async(runtime()).iter(|| async {
            let config = ConfigBuilder::default().verbose(false).build().unwrap();
            let registry = Registry::new(config);

            let root = registry.register(233, "root");
            root.instrument(test()).await;
        })
    });
}

fn bench_basic_baseline(c: &mut Criterion) {
    c.bench_function("basic_baseline", |b| {
        b.to_async(runtime()).iter(|| async {
            let config = ConfigBuilder::default().verbose(false).build().unwrap();
            let registry = Registry::new(config);

            let root = registry.register(233, "root");
            black_box(root);
            test_baseline().await
        })
    });
}

criterion_group!(benches, bench_basic, bench_basic_baseline);

// with_register_to_root   time:   [15.993 ms 16.122 ms 16.292 ms]
// baseline                time:   [13.940 ms 13.961 ms 13.982 ms]

fn bench_many_baseline(c: &mut Criterion) {
    c.bench_function("with_register_to_root_baseline", |b| {
        b.to_async(runtime())
            .iter(|| async { black_box(spawn_many_baseline(10000)).await })
    });
}

fn bench_many_exp(c: &mut Criterion) {
    c.bench_function("with_register_to_root", |b| {
        b.to_async(runtime())
            .iter(|| async { black_box(spawn_many(10000)).await })
    });
}

criterion_group!(
    name = bench_many;
    config = Criterion::default().sample_size(50);
    targets = bench_many_exp, bench_many_baseline
);

criterion_main!(benches, bench_many);
