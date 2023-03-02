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

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use indextree::NodeId;
use pin_project::{pin_project, pinned_drop};

use crate::context::{try_with_context, with_context, ContextId};
use crate::Span;

enum State {
    Initial(Span),
    Polled {
        this_node: NodeId,
        this_context: ContextId,
    },
    Ready,
    /// This span is disabled due to `verbose` configuration.
    Disabled,
}

/// The future for [`InstrumentAwait`][ia].
///
/// [ia]: crate::InstrumentAwait
#[pin_project(PinnedDrop)]
pub struct Instrumented<F: Future, const VERBOSE: bool> {
    #[pin]
    inner: F,
    state: State,
}

impl<F: Future, const VERBOSE: bool> Instrumented<F, VERBOSE> {
    pub(crate) fn new(inner: F, span: Span) -> Self {
        Self {
            inner,
            state: State::Initial(span),
        }
    }
}

impl<F: Future, const VERBOSE: bool> Future for Instrumented<F, VERBOSE> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        // For assertion.
        let old_current = if cfg!(debug_assertions) {
            try_with_context(|c| c.tree().current())
        } else {
            None
        };

        let this_node = match this.state {
            State::Initial(span) => {
                match try_with_context(|c| (c.id(), c.verbose() >= VERBOSE)) {
                    // The tracing for this span is disabled according to the verbose configuration.
                    Some((_, false)) => {
                        *this.state = State::Disabled;
                        return this.inner.poll(cx);
                    }
                    // First polled
                    Some((current_context, true)) => {
                        // First polled, push a new span to the context.
                        let node = with_context(|c| c.tree().push(std::mem::take(span)));
                        *this.state = State::Polled {
                            this_node: node,
                            this_context: current_context,
                        };
                        node
                    }
                    // Not in a context
                    None => return this.inner.poll(cx),
                }
            }
            State::Polled {
                this_node,
                this_context,
            } => {
                match try_with_context(|c| c.id()) {
                    // Context correct
                    Some(current_context) if current_context == *this_context => {
                        // Polled before, just step in.
                        with_context(|c| c.tree().step_in(*this_node));
                        *this_node
                    }
                    // Context changed
                    Some(_) => {
                        tracing::warn!(
                            "future polled in a different context as it was first polled"
                        );
                        return this.inner.poll(cx);
                    }
                    // Out of context
                    None => {
                        tracing::warn!(
                            "future polled not in a context, while it was when first polled"
                        );
                        return this.inner.poll(cx);
                    }
                }
            }
            State::Ready => unreachable!("the instrumented future should always be fused"),
            State::Disabled => return this.inner.poll(cx),
        };

        // The current node must be the this_node.
        debug_assert_eq!(this_node, with_context(|c| c.tree().current()));

        let r = match this.inner.poll(cx) {
            // The future is ready, clean-up this span by popping from the context.
            Poll::Ready(output) => {
                with_context(|c| c.tree().pop());
                *this.state = State::Ready;
                Poll::Ready(output)
            }
            // Still pending, just step out.
            Poll::Pending => {
                with_context(|c| c.tree().step_out());
                Poll::Pending
            }
        };

        // The current node must be the same as we started with.
        debug_assert_eq!(old_current.unwrap(), with_context(|c| c.tree().current()));

        r
    }
}

#[pinned_drop]
impl<F: Future, const VERBOSE: bool> PinnedDrop for Instrumented<F, VERBOSE> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        let current_context = || try_with_context(|c| c.id());

        match this.state {
            State::Polled {
                this_node,
                this_context,
            } => match current_context() {
                // Context correct
                Some(current_context) if current_context == *this_context => {
                    with_context(|c| c.tree().remove_and_detach(*this_node));
                }
                // Context changed
                Some(_) => {
                    tracing::warn!("future is dropped in a different context as it was first polled, cannot clean up!");
                }
                // Out of context
                None => {
                    tracing::warn!("future is not in a context, while it was when first polled, cannot clean up!");
                }
            },
            State::Initial(_) | State::Ready | State::Disabled => {}
        }
    }
}
