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

#[cfg(feature = "stream")]
impl TreeRoot {
    /// Instrument the given stream with the context of this tree root.
    pub fn instrument_stream<S: futures_core::Stream>(
        self,
        stream: S,
    ) -> impl futures_core::Stream<Item = S::Item> {
        #[pin_project::pin_project]
        struct StreamWithContext<S: futures_core::Stream> {
            #[pin]
            inner: S,
            context: Arc<TreeContext>,
        }

        impl<S: futures_core::Stream> futures_core::Stream for StreamWithContext<S> {
            type Item = S::Item;

            fn poll_next(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Option<Self::Item>> {
                let this = self.project();
                CONTEXT.sync_scope(this.context.clone(), || this.inner.poll_next(cx))
            }
        }

        StreamWithContext {
            inner: stream,
            context: self.context,
        }
    }
}

/// The registry of multiple await-trees.
#[derive(Debug)]
pub struct Registry<K> {
    contexts: WeakValueHashMap<K, Weak<TreeContext>>,
    config: Config,
}

impl<K> Registry<K>
where
    K: std::hash::Hash + Eq + std::fmt::Debug,
{
    /// Create a new registry with given `config`.
    pub fn new(config: Config) -> Self {
        Self {
            contexts: WeakValueHashMap::new(),
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
        let context = Arc::new(TreeContext::new(root_span.into(), self.config.verbose));
        self.contexts.insert(key, Arc::clone(&context));

        TreeRoot { context }
    }

    /// Iterate over the clones of all registered await-trees.
    pub fn iter(&self) -> impl Iterator<Item = (&K, Tree)> {
        self.contexts.iter().map(|(k, v)| (k, v.tree().clone()))
    }

    /// Get a clone of the await-tree with given key.
    ///
    /// Returns `None` if the key does not exist or the tree root has been dropped.
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<Tree>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.contexts.get(k).map(|v| v.tree().clone())
    }

    /// Remove all the registered await-trees.
    pub fn clear(&mut self) {
        self.contexts.clear();
    }
}
