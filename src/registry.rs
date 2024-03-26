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

use std::borrow::Borrow;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use derive_builder::Builder;
use parking_lot::RwLock;
use weak_table::WeakValueHashMap;

use crate::context::{Tree, TreeContext, CONTEXT};
use crate::Span;

/// Configuration for an await-tree registry, which affects the behavior of all await-trees in the
/// registry.
#[derive(Debug, Clone, Builder)]
#[builder(default)]
pub struct Config {
    /// Whether to include the **verbose** span in the await-tree.
    verbose: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self { verbose: false }
    }
}

/// The root of an await-tree.
pub struct TreeRoot {
    context: Arc<TreeContext>,
}

impl TreeRoot {
    /// Instrument the given future with the context of this tree root.
    pub async fn instrument<F: Future>(self, future: F) -> F::Output {
        CONTEXT.scope(self.context, future).await
    }
}

type Contexts<K> = WeakValueHashMap<K, Weak<TreeContext>>;

/// The registry of multiple await-trees.
#[derive(Debug)]
pub struct Registry<K> {
    contexts: RwLock<Contexts<K>>,
    config: Config,
}

impl<K> Registry<K>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
{
    /// Create a new registry with given `config`.
    pub fn new(config: Config) -> Self {
        Self {
            contexts: WeakValueHashMap::new().into(),
            config,
        }
    }
}

impl<K> Registry<K>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
{
    /// Register with given key. Returns a [`TreeRoot`] that can be used to instrument a future.
    ///
    /// If the key already exists, a new [`TreeRoot`] is returned and the reference to the old
    /// [`TreeRoot`] is dropped.
    pub fn register(&self, key: K, root_span: impl Into<Span>) -> TreeRoot {
        let context = Arc::new(TreeContext::new(root_span.into(), self.config.verbose));
        self.contexts.write().insert(key, Arc::clone(&context));

        TreeRoot { context }
    }

    /// Get a clone of the await-tree with given key.
    ///
    /// Returns `None` if the key does not exist or the tree root has been dropped.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<Tree>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.contexts.read().get(k).map(|v| v.tree().clone())
    }

    /// Remove all the registered await-trees.
    pub fn clear(&self) {
        self.contexts.write().clear();
    }
}

impl<K> Registry<K>
where
    K: Clone,
{
    /// Collect the snapshots of all await-trees in the registry.
    pub fn collect(&self) -> Vec<(K, Tree)> {
        self.contexts
            .read()
            .iter()
            .map(|(k, v)| (k.to_owned(), v.tree().clone()))
            .collect()
    }
}
