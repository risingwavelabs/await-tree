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

//! Instrument await-tree for actor-based applications.

#![forbid(missing_docs)]

use std::future::Future;

mod context;
mod future;
mod registry;
mod utils;

pub use context::{current_tree, TreeContext};
use flexstr::SharedStr;
pub use future::Instrumented;
pub use registry::{AnyKey, Config, ConfigBuilder, ConfigBuilderError, Key, Registry, TreeRoot};

/// A cheaply cloneable span in the await-tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span(SharedStr);

impl Span {
    pub(crate) fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl<S: AsRef<str>> From<S> for Span {
    fn from(value: S) -> Self {
        Self(SharedStr::from_ref(value))
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Attach spans to a future to be traced in the await-tree.
pub trait InstrumentAwait: Future + Sized {
    /// Instrument the future with a span.
    fn instrument_await(self, span: impl Into<Span>) -> Instrumented<Self, false> {
        Instrumented::new(self, span.into())
    }

    /// Instrument the future with a verbose span, which is optionally enabled based on the registry
    /// configuration.
    fn verbose_instrument_await(self, span: impl Into<Span>) -> Instrumented<Self, true> {
        Instrumented::new(self, span.into())
    }
}
impl<F> InstrumentAwait for F where F: Future {}

#[cfg(test)]
mod tests;
