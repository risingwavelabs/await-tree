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

use std::fmt::{Debug, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time;

use colored::Colorize;
use indextree::{Arena, NodeId};
use itertools::Itertools;
use parking_lot::{Mutex, MutexGuard};

use crate::Span;

/// Node in the span tree.
#[derive(Debug, Clone)]
struct SpanNode {
    /// The span value.
    span: Span,

    /// The time when this span was started, or the future was first polled.
    start_time: coarsetime::Instant,
}

impl SpanNode {
    /// Create a new node with the given value.
    fn new(span: Span) -> Self {
        Self {
            span,
            start_time: coarsetime::Instant::now(),
        }
    }
}

/// The id of an await-tree context. We will check the id recorded in the instrumented future
/// against the current task-local context before trying to update the tree.
pub(crate) type ContextId = u64;

/// An await-tree for a task.
#[derive(Debug, Clone)]
pub struct Tree {
    /// The arena for allocating span nodes in this context.
    arena: Arena<SpanNode>,

    /// The root span node.
    root: NodeId,

    /// The current span node. This is the node that is currently being polled.
    current: NodeId,

    /// Whether to coloring the terminal
    colored: bool,

    /// if the time of execution is beyond it, warn it
    warn_threshold: time::Duration,
}

impl std::fmt::Display for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.fmt_node(f, &self.arena, self.root, 0)?;

        // Format all detached spans.
        for node in self.arena.iter().filter(|n| !n.is_removed()) {
            let id = self.arena.get_node_id(node).unwrap();
            if id == self.root {
                continue;
            }
            if node.parent().is_none() {
                writeln!(f, "[Detached {id}]")?;
                self.fmt_node(f, &self.arena, id, 1)?;
            }
        }

        Ok(())
    }
}

impl Tree {
    /// Get the count of active span nodes in this context.
    #[cfg(test)]
    pub(crate) fn active_node_count(&self) -> usize {
        self.arena.iter().filter(|n| !n.is_removed()).count()
    }

    /// Get the count of active detached span nodes in this context.
    #[cfg(test)]
    pub(crate) fn detached_node_count(&self) -> usize {
        self.arena
            .iter()
            .filter(|n| {
                !n.is_removed()
                    && n.parent().is_none()
                    && self.arena.get_node_id(n).unwrap() != self.root
            })
            .count()
    }

    /// Push a new span as a child of current span, used for future firstly polled.
    ///
    /// Returns the new current span.
    pub(crate) fn push(&mut self, span: Span) -> NodeId {
        let child = self.arena.new_node(SpanNode::new(span));
        self.current.prepend(child, &mut self.arena);
        self.current = child;
        child
    }

    /// Step in the current span to the given child, used for future polled again.
    ///
    /// If the child is not actually a child of the current span, it means we are using a new future
    /// to poll it, so we need to detach it from the previous parent, and attach it to the current
    /// span.
    pub(crate) fn step_in(&mut self, child: NodeId) {
        if !self.current.children(&self.arena).contains(&child) {
            // Actually we can always call this even if `child` is already a child of `current`. But
            // checking first performs better.
            self.current.prepend(child, &mut self.arena);
        }
        self.current = child;
    }

    /// Pop the current span to the parent, used for future ready.
    ///
    /// Note that there might still be some children of this node, like `select_stream.next()`.
    /// The children might be polled again later, and will be attached as the children of a new
    /// span.
    pub(crate) fn pop(&mut self) {
        let parent = self.arena[self.current]
            .parent()
            .expect("the root node should not be popped");
        self.remove_and_detach(self.current);
        self.current = parent;
    }

    /// Step out the current span to the parent, used for future pending.
    pub(crate) fn step_out(&mut self) {
        let parent = self.arena[self.current]
            .parent()
            .expect("the root node should not be stepped out");
        self.current = parent;
    }

    /// Remove the current span and detach the children, used for future aborting.
    ///
    /// The children might be polled again later, and will be attached as the children of a new
    /// span.
    pub(crate) fn remove_and_detach(&mut self, node: NodeId) {
        node.detach(&mut self.arena);
        // Removing detached `node` makes children detached.
        node.remove(&mut self.arena);
    }

    /// Get the current span node id.
    pub(crate) fn current(&self) -> NodeId {
        self.current
    }

    fn fmt_node(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        arena: &Arena<SpanNode>,
        node: NodeId,
        depth: usize,
    ) -> std::fmt::Result {
        f.write_str(&" ".repeat(depth * 2))?;

        let inner = arena[node].get();
        f.write_str(inner.span.as_str())?;

        let elapsed: time::Duration = inner.start_time.elapsed().into();

        let elapsed_str = {
            if depth == 0 {
                "".to_string()
            } else if elapsed.lt(&self.warn_threshold) {
                 format!(" [{:.3?}]", elapsed)
            } else if self.colored {
                 format!(" [{:.3?}]", elapsed).red().to_string()
            } else {
                 format!("!!! [{:.3?}]", elapsed)
            }
        };

        write!(
            f,
            "{}",
            elapsed_str
        )?;

        if depth > 0 && node == self.current {
            f.write_str("  <== current")?;
        }

        f.write_char('\n')?;
        for child in node
            .children(arena)
            .sorted_by_key(|&id| arena[id].get().start_time)
        {
            self.fmt_node(f, arena, child, depth + 1)?;
        }

        Ok(())
    }

}

/// The task-local await-tree context.
#[derive(Debug)]
pub struct TreeContext {
    /// The id of the context.
    id: ContextId,

    /// Whether to include the "verbose" span in the tree.
    verbose: bool,

    /// The await-tree.
    tree: Mutex<Tree>,
}

impl TreeContext {
    /// Create a new context.
    pub(crate) fn new(root_span: Span, verbose: bool, colored: bool, warn_threshold: time::Duration) -> Self {
        static ID: AtomicU64 = AtomicU64::new(0);
        let id = ID.fetch_add(1, Ordering::Relaxed);

        let mut arena = Arena::new();
        let root = arena.new_node(SpanNode::new(root_span));

        Self {
            id,
            verbose,
            tree: Tree {
                colored,
                warn_threshold,
                arena,
                root,
                current: root,
            }
            .into(),
        }
    }

    /// Returns the locked guard of the tree.
    pub(crate) fn tree(&self) -> MutexGuard<'_, Tree> {
        self.tree.lock()
    }

    /// Whether the verbose span should be included.
    pub(crate) fn verbose(&self) -> bool {
        self.verbose
    }
}

/// Public interfaces.
impl TreeContext {
    /// Get the context id.
    pub fn id(&self) -> ContextId {
        self.id
    }
}

tokio::task_local! {
    pub(crate) static CONTEXT: Arc<TreeContext>
}

pub(crate) fn context() -> Option<Arc<TreeContext>> {
    CONTEXT.try_with(Arc::clone).ok()
}

/// Get the await-tree of current task. Returns `None` if we're not instrumented.
///
/// This is useful if you want to check which component or runtime task is calling this function.
pub fn current_tree() -> Option<Tree> {
    context().map(|c| c.tree().clone())
}
