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

use std::any::Any;
use std::fmt::Debug;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Weak};

use derive_builder::Builder;
use parking_lot::RwLock;
use weak_table::WeakValueHashMap;

use crate::context::{Tree, TreeContext, CONTEXT};
use crate::utils::{DynEq, DynHash};
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
    #[allow(dead_code)]
    registry: Weak<RegistryCore>,
}

impl TreeRoot {
    /// Instrument the given future with the context of this tree root.
    pub async fn instrument<F: Future>(self, future: F) -> F::Output {
        CONTEXT.scope(self.context, future).await
    }
}

/// A key that can be used to identify a task and its await-tree in the [`Registry`].
///
/// All thread-safe types that can be used as a key of a hash map are automatically implemented with
/// this trait.
pub trait Key: Hash + Eq + Debug + Send + Sync + 'static {}
impl<T> Key for T where T: Hash + Eq + Debug + Send + Sync + 'static {}

/// The object-safe version of [`Key`], automatically implemented.
trait ObjKey: DynHash + DynEq + Debug + Send + Sync + 'static {}
impl<T> ObjKey for T where T: DynHash + DynEq + Debug + Send + Sync + 'static {}

/// Type-erased key for the [`Registry`].
#[derive(Debug, Clone)]
pub struct AnyKey(Arc<dyn ObjKey>);

impl PartialEq for AnyKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other.0.as_dyn_eq())
    }
}

impl Eq for AnyKey {}

impl Hash for AnyKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl AnyKey {
    fn new(key: impl ObjKey) -> Self {
        Self(Arc::new(key))
    }

    /// Cast the key to `dyn Any`.
    pub fn as_any(&self) -> &dyn Any {
        self.0.as_ref().as_any()
    }
}

type Contexts = RwLock<WeakValueHashMap<AnyKey, Weak<TreeContext>>>;

#[derive(Debug)]
struct RegistryCore {
    contexts: Contexts,
    config: Config,
}

/// The registry of multiple await-trees.
#[derive(Debug)]
pub struct Registry(Arc<RegistryCore>);

impl Clone for Registry {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Registry {
    fn contexts(&self) -> &Contexts {
        &self.0.contexts
    }

    fn config(&self) -> &Config {
        &self.0.config
    }
}

impl Registry {
    /// Create a new registry with given `config`.
    pub fn new(config: Config) -> Self {
        Self(
            RegistryCore {
                contexts: Default::default(),
                config,
            }
            .into(),
        )
    }

    /// Register with given key. Returns a [`TreeRoot`] that can be used to instrument a future.
    ///
    /// If the key already exists, a new [`TreeRoot`] is returned and the reference to the old
    /// [`TreeRoot`] is dropped.
    pub fn register(&self, key: impl Key, root_span: impl Into<Span>) -> TreeRoot {
        let context = Arc::new(TreeContext::new(root_span.into(), self.config().verbose));
        self.contexts()
            .write()
            .insert(AnyKey::new(key), Arc::clone(&context));

        TreeRoot {
            context,
            registry: Arc::downgrade(&self.0),
        }
    }

    /// Get a clone of the await-tree with given key.
    ///
    /// Returns `None` if the key does not exist or the tree root has been dropped.
    pub fn get(&self, key: impl Key) -> Option<Tree> {
        self.contexts()
            .read()
            .get(&AnyKey::new(key)) // TODO: accept ref can?
            .map(|v| v.tree().clone())
    }

    /// Remove all the registered await-trees.
    pub fn clear(&self) {
        self.contexts().write().clear();
    }

    /// Collect the snapshots of all await-trees with the key of type `K`.
    pub fn collect<K: Key + Clone>(&self) -> Vec<(K, Tree)> {
        self.contexts()
            .read()
            .iter()
            .filter_map(|(k, v)| {
                k.0.as_ref()
                    .as_any()
                    .downcast_ref::<K>()
                    .map(|k| (k.clone(), v.tree().clone()))
            })
            .collect()
    }

    /// Collect the snapshots of all await-trees regardless of the key type.
    pub fn collect_all(&self) -> Vec<(AnyKey, Tree)> {
        self.contexts()
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.tree().clone()))
            .collect()
    }
}
