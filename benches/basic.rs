use await_tree::{ConfigBuilder, InstrumentAwait, Registry};
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

// time:   [6.5488 ms 6.5541 ms 6.5597 ms]
// change: [+6.5978% +6.7838% +6.9299%] (p = 0.00 < 0.05)
// Performance has regressed.
fn bench_basic(c: &mut Criterion) {
    c.bench_function("basic", |b| {
        b.to_async(runtime()).iter(|| async {
            let config = ConfigBuilder::default().verbose(false).build().unwrap();
            let mut mgr = Registry::new(config);

            let root = mgr.register(233, "root");
            root.instrument(test()).await;
        })
    });
}

criterion_group!(benches, bench_basic);
criterion_main!(benches);
