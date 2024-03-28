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

use crate::context::ContextId;
use crate::root::current_context;
use crate::Span;

enum State {
    Initial(Span),
    Polled {
        this_node: NodeId,
        this_context_id: ContextId,
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
        let context = current_context();

        let (context, this_node) = match this.state {
            State::Initial(span) => {
                match context {
                    Some(c) => {
                        if !c.verbose() && VERBOSE {
                            // The tracing for this span is disabled according to the verbose
                            // configuration.
                            *this.state = State::Disabled;
                            return this.inner.poll(cx);
                        }
                        // First polled, push a new span to the context.
                        let node = c.tree().push(std::mem::take(span));
                        *this.state = State::Polled {
                            this_node: node,
                            this_context_id: c.id(),
                        };
                        (c, node)
                    }
                    // Not in a context
                    None => return this.inner.poll(cx),
                }
            }
            State::Polled {
                this_node,
                this_context_id: this_context,
            } => {
                match context {
                    // Context correct
                    Some(c) if c.id() == *this_context => {
                        // Polled before, just step in.
                        c.tree().step_in(*this_node);
                        (c, *this_node)
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
        debug_assert_eq!(this_node, context.tree().current());

        match this.inner.poll(cx) {
            // The future is ready, clean-up this span by popping from the context.
            Poll::Ready(output) => {
                context.tree().pop();
                *this.state = State::Ready;
                Poll::Ready(output)
            }
            // Still pending, just step out.
            Poll::Pending => {
                context.tree().step_out();
                Poll::Pending
            }
        }
    }
}

#[pinned_drop]
impl<F: Future, const VERBOSE: bool> PinnedDrop for Instrumented<F, VERBOSE> {
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();

        match this.state {
            State::Polled {
                this_node,
                this_context_id,
            } => match current_context() {
                // Context correct
                Some(c) if c.id() == *this_context_id => {
                    c.tree().remove_and_detach(*this_node);
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
