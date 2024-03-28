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

use futures::future::pending;
use tokio::time::sleep;

use crate::{Config, InstrumentAwait, Registry};

#[tokio::test]
async fn main() {
    let registry = Registry::new(Config::default());

    tokio::spawn(registry.register((), "root").instrument(async {
        crate::spawn_anonymous("child", async {
            crate::spawn_anonymous("grandson", async {
                pending::<()>().await;
            })
            .instrument_await("wait for grandson")
            .await
            .unwrap()
        })
        .instrument_await("wait for child")
        .await
        .unwrap()
    }));

    sleep(Duration::from_secs(1)).await;

    assert_eq!(registry.collect::<()>().len(), 1);
    assert_eq!(registry.collect_anonymous().len(), 2);
    assert_eq!(registry.collect_all().len(), 3);
}
