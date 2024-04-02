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

// TODO: should we consider exposing `current_registry`
// so that users can not only spawn tasks but also get and collect trees?

// TODO: should we support "global registry" for users to quick start?

use std::future::Future;

use tokio::task::JoinHandle;

use crate::{Key, Registry, Span};

/// Spawns a new asynchronous task instrumented with the given root [`Span`], returning a
/// [`JoinHandle`] for it.
///
/// The spawned task will be registered in the current [`Registry`](crate::Registry) returned by
/// [`Registry::current`] with the given [`Key`], if it exists. Otherwise, this is equivalent to
/// [`tokio::spawn`].
pub fn spawn<T>(key: impl Key, root_span: impl Into<Span>, future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    if let Some(registry) = Registry::try_current() {
        tokio::spawn(registry.register(key, root_span).instrument(future))
    } else {
        tokio::spawn(future)
    }
}

/// Spawns a new asynchronous task instrumented with the given root [`Span`], returning a
/// [`JoinHandle`] for it.
///
/// The spawned task will be registered in the current [`Registry`](crate::Registry) returned by
/// [`Registry::current`] , if it exists. Otherwise, this is equivalent to [`tokio::spawn`].
pub fn spawn_anonymous<T>(root_span: impl Into<Span>, future: T) -> JoinHandle<T::Output>
where
    T: Future + Send + 'static,
    T::Output: Send + 'static,
{
    if let Some(registry) = Registry::try_current() {
        tokio::spawn(registry.register_anonymous(root_span).instrument(future))
    } else {
        tokio::spawn(future)
    }
}
