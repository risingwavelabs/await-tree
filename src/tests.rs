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

use futures::future::{join_all, poll_fn, select_all};
use futures::{pin_mut, FutureExt, Stream, StreamExt};
use itertools::Itertools;

use crate::context::with_context;
use crate::{Config, InstrumentAwait, Registry};

async fn sleep(time: u64) {
    tokio::time::sleep(std::time::Duration::from_millis(time)).await;
    println!("slept {time}ms");
}

async fn sleep_nested() {
    join_all([
        sleep(1500).instrument_await("sleep nested 1500"),
        sleep(2500).instrument_await("sleep nested 2500"),
    ])
    .await;
}

async fn multi_sleep() {
    sleep(400).await;

    sleep(800)
        .instrument_await("sleep another in multi sleep")
        .await;
}

fn stream1() -> impl Stream<Item = ()> {
    use futures::stream::{iter, once};

    iter(std::iter::repeat_with(|| {
        once(async {
            sleep(150).await;
        })
    }))
    .flatten()
}

fn stream2() -> impl Stream<Item = ()> {
    use futures::stream::{iter, once};

    iter([
        once(async {
            sleep(444).await;
        })
        .boxed(),
        once(async {
            join_all([
                sleep(400).instrument_await("sleep nested 400"),
                sleep(600).instrument_await("sleep nested 600"),
            ])
            .await;
        })
        .boxed(),
    ])
    .flatten()
}

async fn hello() {
    async move {
        // Join
        join_all([
            sleep(1000)
                .boxed()
                .instrument_await(format!("sleep {}", 1000)),
            sleep(2000).boxed().instrument_await("sleep 2000"),
            sleep_nested().boxed().instrument_await("sleep nested"),
            multi_sleep().boxed().instrument_await("multi sleep"),
        ])
        .await;

        // Join another
        join_all([
            sleep(1200).instrument_await("sleep 1200"),
            sleep(2200).instrument_await("sleep 2200"),
        ])
        .await;

        // Cancel
        select_all([
            sleep(666).boxed().instrument_await("sleep 666"),
            sleep_nested()
                .boxed()
                .instrument_await("sleep nested (should be cancelled)"),
        ])
        .await;

        // Check whether cleaned up
        sleep(233).instrument_await("sleep 233").await;

        // Check stream next drop
        {
            let mut stream1 = stream1().fuse().boxed();
            let mut stream2 = stream2().fuse().boxed();
            let mut count = 0;

            'outer: loop {
                tokio::select! {
                    _ = stream1.next().instrument_await(format!("stream1 next {count}")) => {},
                    r = stream2.next().instrument_await(format!("stream2 next {count}")) => {
                        if r.is_none() { break 'outer }
                    },
                }
                sleep(50)
                    .instrument_await(format!("sleep before next stream poll: {count}"))
                    .await;
                count += 1;
            }
        }

        // Check whether cleaned up
        sleep(233).instrument_await("sleep 233").await;

        // TODO: add tests on sending the future to another task or context.
    }
    .instrument_await("hello")
    .await;

    // Aborted futures have been cleaned up. There should only be a single active node of root.
    assert_eq!(with_context(|c| c.active_node_count()), 1);
}

#[tokio::test]
async fn test_await_tree() {
    let mut registry = Registry::new(Config::default());
    let root = registry.register(233, "actor 233");

    let fut = root.instrument(hello());
    pin_mut!(fut);

    let expected_counts = vec![
        (1, 0),
        (8, 0),
        (9, 0),
        (8, 0),
        (6, 0),
        (5, 0),
        (4, 0),
        (4, 0),
        (3, 0),
        (6, 0),
        (3, 0),
        (4, 0),
        (3, 0),
        (4, 0),
        (3, 0),
        (4, 0),
        (3, 0),
        (6, 0),
        (5, 2),
        (6, 0),
        (5, 2),
        (6, 0),
        (5, 0),
        (4, 1),
        (5, 0),
        (3, 0),
        (3, 0),
    ];
    let mut actual_counts = vec![];

    poll_fn(|cx| {
        let tree = registry.iter().exactly_one().ok().unwrap().1;
        println!("{tree}");
        actual_counts.push((tree.active_node_count(), tree.detached_node_count()));
        fut.poll_unpin(cx)
    })
    .await;

    assert_eq!(actual_counts, expected_counts);
}
