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

//! Generate accurate and informative tree dumps of asynchronous tasks.
//!
//! # Example
//!
//! Below is a basic example of how to trace asynchronous tasks with the global registry of the
//! `await-tree` crate.
//!
//! ```rust
//! # use std::time::Duration;
//! # use tokio::time::sleep;
//! # use await_tree::{InstrumentAwait, Registry, span};
//! # use futures::future::join;
//! #
//! # async fn work() { futures::future::pending::<()>().await }
//! #
//! async fn bar(i: i32) {
//!     // `&'static str` span
//!     baz(i).instrument_await("baz in bar").await
//! }
//!
//! async fn baz(i: i32) {
//!     // runtime `String` span is also supported
//!     work().instrument_await(span!("working in baz {i}")).await
//! }
//!
//! async fn foo() {
//!     // spans of joined futures will be siblings in the tree
//!     join(
//!         bar(3).instrument_await("bar"),
//!         baz(2).instrument_await("baz"),
//!     )
//!     .await;
//! }
//!
//! # #[tokio::main]
//! # async fn main() {
//! // Init the global registry to start tracing the tasks.
//! await_tree::init_global_registry(Default::default());
//! // Spawn a task with root span "foo" and key "foo".
//! await_tree::spawn("foo", "foo", foo());
//! // Let the tasks run for a while.
//! sleep(Duration::from_secs(1)).await;
//! // Get the tree of the task with key "foo".
//! let tree = Registry::current().get("foo").unwrap();
//!
//! // foo [1.006s]
//! //   bar [1.006s]
//! //     baz in bar [1.006s]
//! //       working in baz 3 [1.006s]
//! //   baz [1.006s]
//! //     working in baz 2 [1.006s]
//! println!("{tree}");
//! # }
//! ```

#![forbid(missing_docs)]

use std::future::Future;

mod context;
mod future;
mod global;
mod obj_utils;
mod registry;
mod root;
mod span;
#[cfg(feature = "tokio")]
mod spawn;

pub use context::{current_tree, Tree};
pub use future::Instrumented;
pub use global::init_global_registry;
pub use registry::{AnyKey, Config, ConfigBuilder, ConfigBuilderError, Key, Registry, ToRootSpan};
pub use root::TreeRoot;
pub use span::{Span, SpanExt};
#[cfg(feature = "tokio")]
pub use spawn::{spawn, spawn_anonymous, spawn_derived_root};

#[cfg(feature = "macros")]
pub use await_tree_macros::instrument;

#[doc(hidden)]
pub mod __private {
    pub use crate::span::fmt_span;
}

/// Attach spans to a future to be traced in the await-tree.
pub trait InstrumentAwait: Future + Sized {
    /// Instrument the future with a span.
    fn instrument_await(self, span: impl Into<Span>) -> Instrumented<Self> {
        Instrumented::new(self, span.into())
    }
}
impl<F> InstrumentAwait for F where F: Future {}

#[cfg(test)]
mod tests;
