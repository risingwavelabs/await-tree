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
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::sync::{Arc, Weak};

use derive_builder::Builder;
use parking_lot::RwLock;
use weak_table::WeakValueHashMap;

use crate::context::{ContextId, Tree, TreeContext};
use crate::obj_utils::{DynEq, DynHash};
use crate::{Span, TreeRoot};

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

/// A key that can be used to identify a task and its await-tree in the [`Registry`].
///
/// All thread-safe types that can be used as a key of a hash map are automatically implemented with
/// this trait.
pub trait Key: Hash + Eq + Debug + Send + Sync + 'static {}
impl<T> Key for T where T: Hash + Eq + Debug + Send + Sync + 'static {}

/// The object-safe version of [`Key`], automatically implemented.
trait ObjKey: DynHash + DynEq + Debug + Send + Sync + 'static {}
impl<T> ObjKey for T where T: DynHash + DynEq + Debug + Send + Sync + 'static {}

/// Key type for anonymous await-trees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AnonymousKey(ContextId);

impl Display for AnonymousKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Anonymous #{}", self.0 .0)
    }
}

/// Type-erased key for the [`Registry`].
#[derive(Clone)]
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

impl Debug for AnyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for AnyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO: for all `impl Display`?
        macro_rules! delegate_to_display {
            ($($t:ty),* $(,)?) => {
                $(
                    if let Some(k) = self.as_any().downcast_ref::<$t>() {
                        return write!(f, "{}", k);
                    }
                )*
            };
        }
        delegate_to_display!(String, &str, AnonymousKey);

        write!(f, "{:?}", self)
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

    /// Returns whether the key is of type `K`.
    ///
    /// Equivalent to `self.as_any().is::<K>()`.
    pub fn is<K: Any>(&self) -> bool {
        self.as_any().is::<K>()
    }

    /// Returns whether the key corresponds to an anonymous await-tree.
    pub fn is_anonymous(&self) -> bool {
        self.as_any().is::<AnonymousKey>()
    }

    /// Returns the key as a reference to type `K`, if it is of type `K`.
    ///
    /// Equivalent to `self.as_any().downcast_ref::<K>()`.
    pub fn downcast_ref<K: Any>(&self) -> Option<&K> {
        self.as_any().downcast_ref()
    }
}

type Contexts = RwLock<WeakValueHashMap<AnyKey, Weak<TreeContext>>>;

struct RegistryCore {
    contexts: Contexts,
    config: Config,
}

/// The registry of multiple await-trees.
///
/// Can be cheaply cloned to share the same registry.
pub struct Registry(Arc<RegistryCore>);

impl Debug for Registry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("config", self.config())
            .finish_non_exhaustive()
    }
}

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

    /// Returns the current registry, if exists.
    ///
    /// 1. If the current task is registered with a registry, returns the registry.
    /// 2. If the global registry is initialized with
    ///    [`init_global_registry`](crate::global::init_global_registry), returns the global
    ///    registry.
    /// 3. Otherwise, returns `None`.
    pub fn try_current() -> Option<Self> {
        crate::root::current_registry()
    }

    /// Returns the current registry, panics if not exists.
    ///
    /// See [`Registry::try_current`] for more information.
    pub fn current() -> Self {
        Self::try_current().expect("no current registry")
    }

    fn register_inner(&self, key: impl Key, context: Arc<TreeContext>) -> TreeRoot {
        self.contexts()
            .write()
            .insert(AnyKey::new(key), Arc::clone(&context));

        TreeRoot {
            context,
            registry: WeakRegistry(Arc::downgrade(&self.0)),
        }
    }

    /// Register with given key. Returns a [`TreeRoot`] that can be used to instrument a future.
    ///
    /// If the key already exists, a new [`TreeRoot`] is returned and the reference to the old
    /// [`TreeRoot`] is dropped.
    pub fn register(&self, key: impl Key, root_span: impl Into<Span>) -> TreeRoot {
        let context = Arc::new(TreeContext::new(root_span.into(), self.config().verbose));
        self.register_inner(key, context)
    }

    /// Register an anonymous await-tree without specifying a key. Returns a [`TreeRoot`] that can
    /// be used to instrument a future.
    ///
    /// Anonymous await-trees are not able to be retrieved through the [`Registry::get`] method. Use
    /// [`Registry::collect_anonymous`] or [`Registry::collect_all`] to collect them.
    // TODO: we have keyed and anonymous, should we also have a typed-anonymous (for classification
    // only)?
    pub fn register_anonymous(&self, root_span: impl Into<Span>) -> TreeRoot {
        let context = Arc::new(TreeContext::new(root_span.into(), self.config().verbose));
        self.register_inner(AnonymousKey(context.id()), context) // use the private id as the key
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

    /// Collect the snapshots of all await-trees registered with [`Registry::register_anonymous`].
    pub fn collect_anonymous(&self) -> Vec<Tree> {
        self.contexts()
            .read()
            .iter()
            .filter_map(|(k, v)| {
                if k.is_anonymous() {
                    Some(v.tree().clone())
                } else {
                    None
                }
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

pub(crate) struct WeakRegistry(Weak<RegistryCore>);

impl WeakRegistry {
    pub fn upgrade(&self) -> Option<Registry> {
        self.0.upgrade().map(Registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = Registry::new(Config::default());

        let _0_i32 = registry.register(0_i32, "0");
        let _1_i32 = registry.register(1_i32, "1");
        let _2_i32 = registry.register(2_i32, "2");

        let _0_str = registry.register("0", "0");
        let _1_str = registry.register("1", "1");

        let _unit = registry.register((), "()");
        let _unit_replaced = registry.register((), "[]");

        let _anon = registry.register_anonymous("anon");
        let _anon = registry.register_anonymous("anon");

        let i32s = registry.collect::<i32>();
        assert_eq!(i32s.len(), 3);

        let strs = registry.collect::<&'static str>();
        assert_eq!(strs.len(), 2);

        let units = registry.collect::<()>();
        assert_eq!(units.len(), 1);

        let anons = registry.collect_anonymous();
        assert_eq!(anons.len(), 2);

        let all = registry.collect_all();
        assert_eq!(all.len(), 8);
    }
}
