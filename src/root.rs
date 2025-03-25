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

use std::future::Future;
use std::sync::Arc;

use crate::context::TreeContext;
use crate::global::global_registry;
use crate::registry::WeakRegistry;
use crate::Registry;

/// The root of an await-tree.
pub struct TreeRoot {
    pub(crate) context: Arc<TreeContext>,
    pub(crate) registry: WeakRegistry,
}

task_local::task_local! {
    static ROOT: TreeRoot
}

pub(crate) fn current_context() -> Option<Arc<TreeContext>> {
    ROOT.try_with(|r| r.context.clone()).ok()
}

pub(crate) fn current_registry() -> Option<Registry> {
    let local = || ROOT.try_with(|r| r.registry.upgrade()).ok().flatten();
    let global = global_registry;

    local().or_else(global)
}

impl TreeRoot {
    /// Instrument the given future with the context of this tree root.
    pub async fn instrument<F: Future>(self, future: F) -> F::Output {
        ROOT.scope(self, future).await
    }
}
