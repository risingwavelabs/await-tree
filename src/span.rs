// Copyright 2025 RisingWave Labs
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

type SpanName = flexstr::SharedStr;

/// A cheaply cloneable span in the await-tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    name: SpanName,
    is_verbose: bool,
    is_long_running: bool,
}

impl Span {
    /// Set the verbose status of the span.
    pub const fn verbose(mut self) -> Self {
        self.is_verbose = true;
        self
    }

    /// Set the long-running status of the span.
    pub const fn long_running(mut self) -> Self {
        self.is_long_running = true;
        self
    }
}

impl Span {
    pub(crate) fn as_str(&self) -> &str {
        self.name.as_str()
    }
}

impl<S: AsRef<str>> From<S> for Span {
    fn from(value: S) -> Self {
        Self {
            name: SpanName::from_ref(value),
            is_long_running: false,
            is_verbose: false,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}
