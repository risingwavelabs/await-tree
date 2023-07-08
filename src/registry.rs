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
use std::collections::HashMap;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Weak};
use std::time;

use derive_builder::Builder;

use crate::context::{Tree, TreeContext, CONTEXT};
use crate::Span;

/// Configuration for an await-tree registry, which affects the behavior of all await-trees in the
/// registry.
#[derive(Debug, Clone, Builder)]
#[builder(default)]
pub struct Config {
    /// Whether to include the **verbose** span in the await-tree.
    pub verbose: bool,

    /// Whether to coloring the terminal
    pub colored : bool,

    /// if the time of execution is beyond it, warn it
    pub warn_threshold : time::Duration,
}

#[allow(clippy::derivable_impls)]
impl Default for Config {
    fn default() -> Self {
        Self {
            verbose: false,
            colored: false,
            warn_threshold : time::Duration::from_secs(10)
        }
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

/// The registry of multiple await-trees.
#[derive(Debug)]
pub struct Registry<K> {
    contexts: HashMap<K, Weak<TreeContext>>,
    config: Config,
}

impl<K> Registry<K> {
    /// Create a new registry with given `config`.
    pub fn new(config: Config) -> Self {
        Self {
            contexts: HashMap::new(),
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
    pub fn register(&mut self, key: K, root_span: impl Into<Span>) -> TreeRoot {
        // TODO: make this more efficient
        self.contexts.retain(|_, v| v.upgrade().is_some());

        let context = Arc::new(TreeContext::new(root_span.into(), self.config.verbose, self.config.colored, self.config.warn_threshold));
        let weak = Arc::downgrade(&context);
        self.contexts.insert(key, weak);

        TreeRoot { context }
    }

    /// Iterate over the clones of all registered await-trees.
    pub fn iter(&self) -> impl Iterator<Item = (&K, Tree)> {
        self.contexts
            .iter()
            .filter_map(|(k, v)| v.upgrade().map(|v| (k, v.tree().clone())))
    }

    /// Get a clone of the await-tree with given key.
    ///
    /// Returns `None` if the key does not exist or the tree root has been dropped.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<Tree>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.contexts
            .get(k)
            .and_then(|v| v.upgrade())
            .map(|v| v.tree().clone())
    }

    /// Remove all the registered await-trees.
    pub fn clear(&mut self) {
        self.contexts.clear();
    }
}
