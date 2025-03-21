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

#[doc(hidden)]
pub fn fmt_span<'a>(args: std::fmt::Arguments<'a>) -> Span {
    let name = if let Some(str) = args.as_str() {
        SpanName::from_ref(str)
    } else {
        flexstr::flex_fmt(args)
    };
    Span::new(name)
}

/// Creates a new span with formatted name.
///
/// [`instrument_await`] accepts any type that implements [`AsRef<str>`] as the span name.
/// This macro provides similar functionality to [`format!`], but with improved performance
/// by creating the span name on the stack when possible, avoiding unnecessary allocations.
///
/// [`instrument_await`]: crate::InstrumentAwait::instrument_await
#[macro_export]
// XXX: Without this extra binding (`let res = ..`), it will make the future `!Send`.
//      This is also how `std::format!` behaves. But why?
macro_rules! span {
    ($($fmt_arg:tt)*) => {{
        let res = $crate::__private::fmt_span(format_args!($($fmt_arg)*));
        res
    }};
}

/// A cheaply cloneable span in the await-tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub(crate) name: SpanName,
    pub(crate) is_verbose: bool,
    pub(crate) is_long_running: bool,
}

impl Span {
    fn new(name: SpanName) -> Self {
        Self {
            name,
            is_verbose: false,
            is_long_running: false,
        }
    }
}

impl Span {
    /// Set the verbose attribute of the span.
    ///
    /// When a span is marked as verbose, it will be included in the output
    /// only if the `verbose` flag in the [`Config`] is set.
    ///
    /// [`Config`]: crate::Config
    pub fn verbose(mut self) -> Self {
        self.is_verbose = true;
        self
    }

    /// Set the long-running attribute of the span.
    ///
    /// When a span is marked as long-running, it will not be marked as "!!!"
    /// in the formatted [`Tree`] if it takes too long to complete. The root
    /// span is always marked as long-running.
    ///
    /// [`Tree`]: crate::Tree
    pub fn long_running(mut self) -> Self {
        self.is_long_running = true;
        self
    }
}

/// Convert a value into a span and set attributes.
#[easy_ext::ext(SpanExt)]
impl<T: Into<Span>> T {
    /// Convert `self` into a span and set the verbose attribute.
    ///
    /// See [`Span::verbose`] for more details.
    pub fn verbose(self) -> Span {
        self.into().verbose()
    }

    /// Convert `self` into a span and set the long-running attribute.
    ///
    /// See [`Span::long_running`] for more details.
    pub fn long_running(self) -> Span {
        self.into().long_running()
    }
}

impl<S: AsRef<str>> From<S> for Span {
    fn from(value: S) -> Self {
        Self::new(SpanName::from_ref(value))
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}
