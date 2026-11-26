//! A host tree can say where an out-of-flow box was written, and the engine places the box there.
//!
//! A host may keep an absolutely positioned box as a child of its containing block rather than of
//! the box it was written inside, which is what zgui's box tree does. The containing block then
//! records the box's static position after all of its own in-flow children, which is not where the
//! box was written. The host answers [`LayoutTree::static_position`] for it, and a box whose insets
//! in an axis are both `auto` is placed at that answer (css-position-3 §4.6; CSS 2.1 §10.3.7 and
//! §10.6.4).
//!
//! `Host` is a real tree with no stand-ins: it stores styles, children, caches and layouts, and the
//! one thing it adds is the answer the test gives it, which is the hook under test.

use cephal::compute::block::BlockContext;
use cephal::compute::compute_root_layout;
use cephal::compute::oof::AxisStaticPosition;
use cephal::geometry::{AvailableSpace, Line, Point, Size};
use cephal::style::{Dimension, LengthPercentageAuto, Position};
use cephal::tree::{CacheAccess, Layout, LayoutInput, LayoutOutput, LayoutTree, NodeCache, NodeId};
use cephal::Style;

struct Node {
    style: Style,
    children: Vec<NodeId>,
    cache: NodeCache,
    layout: Layout,
}

#[derive(Default)]
struct Host {
    nodes: Vec<Node>,
    speculation: u32,
    /// The block-axis static position the host gives, in its containing block's coordinates.
    written_at: Option<f32>,
    /// What the engine recorded for each box it asked about.
    asked: std::cell::RefCell<Vec<(NodeId, NodeId, f32)>>,
}

impl Host {
    fn add(&mut self, style: Style, children: &[NodeId]) -> NodeId {
        self.nodes.push(Node { style, children: children.to_vec(), cache: NodeCache::default(), layout: Layout::ZERO });
        NodeId::from(self.nodes.len() - 1)
    }
    fn node(&self, id: NodeId) -> &Node {
        &self.nodes[usize::from(id)]
    }
}

impl LayoutTree for Host {
    type ChildIter<'a> = std::iter::Copied<std::slice::Iter<'a, NodeId>>;

    fn children(&self, node: NodeId) -> Self::ChildIter<'_> {
        self.node(node).children.iter().copied()
    }
    fn child_count(&self, node: NodeId) -> usize {
        self.node(node).children.len()
    }
    fn child_at(&self, node: NodeId, index: usize) -> NodeId {
        self.node(node).children[index]
    }
    fn style(&self, node: NodeId) -> &Style {
        &self.node(node).style
    }
    fn set_unrounded_layout(&mut self, node: NodeId, layout: &Layout) {
        if self.speculation == 0 {
            self.nodes[usize::from(node)].layout = *layout;
        }
    }
    fn compute_child_layout(&mut self, node: NodeId, inputs: LayoutInput, block_ctx: Option<&mut BlockContext<'_>>) -> LayoutOutput {
        cephal::compute::compute_child_layout(self, node, inputs, block_ctx)
    }
    fn static_position(&self, cb: NodeId, node: NodeId, recorded: Point<AxisStaticPosition>) -> Point<AxisStaticPosition> {
        self.asked.borrow_mut().push((cb, node, recorded.y.area.start));
        match self.written_at {
            Some(y) => Point { x: recorded.x, y: AxisStaticPosition { area: Line { start: y, end: y }, align: recorded.y.align } },
            None => recorded,
        }
    }
}

impl CacheAccess for Host {
    fn cache(&self, node: NodeId) -> &NodeCache {
        &self.node(node).cache
    }
    fn cache_mut(&mut self, node: NodeId) -> &mut NodeCache {
        &mut self.nodes[usize::from(node)].cache
    }
    fn generation(&self) -> u32 {
        1
    }
    fn speculation(&self) -> u32 {
        self.speculation
    }
    fn set_speculation(&mut self, depth: u32) {
        self.speculation = depth;
    }
}

const VIEWPORT: Size<AvailableSpace> = Size { width: AvailableSpace::Definite(300.0), height: AvailableSpace::MaxContent };

/// A positioned block holding a 30-tall in-flow block and, after it, an absolutely positioned box
/// whose `top` and `bottom` are both `auto`; `left` is 0, so only the block axis reads the static
/// position.
fn tree(host: &mut Host) -> (NodeId, NodeId, NodeId) {
    let mut flow = Style::DEFAULT;
    flow.size.height = Dimension::length(30.0);
    let flow = host.add(flow, &[]);
    let mut row = Style::DEFAULT;
    row.position = Position::Absolute;
    row.inset.left = LengthPercentageAuto::length(0.0);
    row.size = Size { width: Dimension::percent(1.0), height: Dimension::length(24.0) };
    let row = host.add(row, &[]);
    let mut cb = Style::DEFAULT;
    cb.position = Position::Relative;
    let cb = host.add(cb, &[flow, row]);
    (cb, flow, row)
}

#[test]
fn without_an_answer_the_box_sits_after_the_in_flow_children_of_its_containing_block() {
    let mut host = Host::default();
    let (cb, _, row) = tree(&mut host);
    compute_root_layout(&mut host, cb, VIEWPORT);
    assert_eq!(host.node(row).layout.location, Point { x: 0.0, y: 30.0 });
    assert_eq!(host.asked.borrow().as_slice(), &[(cb, row, 30.0)], "asked once, with what the block recorded");
}

#[test]
fn the_box_sits_where_the_host_says_it_was_written() {
    // The host says the box was written before the in-flow block, so its static position is the
    // containing block's content top. Chromium 141 puts an absolutely positioned `top: auto` box
    // written first in its parent at that parent's top.
    let mut host = Host { written_at: Some(0.0), ..Host::default() };
    let (cb, flow, row) = tree(&mut host);
    compute_root_layout(&mut host, cb, VIEWPORT);
    assert_eq!(host.node(row).layout.location, Point { x: 0.0, y: 0.0 });
    assert_eq!(host.node(row).layout.size, Size { width: 300.0, height: 24.0 });
    assert_eq!(host.node(flow).layout.location.y, 0.0, "the in-flow block does not move");
    assert_eq!(host.node(cb).layout.size.height, 30.0, "an out-of-flow box adds no height");
}

#[test]
fn an_inset_in_the_axis_wins_over_the_static_position() {
    // `top: 5px` resolves the axis from the inset; the static position is read only when both
    // insets in an axis are `auto`.
    let mut host = Host { written_at: Some(17.0), ..Host::default() };
    let (cb, _, row) = tree(&mut host);
    host.nodes[usize::from(row)].style.inset.top = LengthPercentageAuto::length(5.0);
    compute_root_layout(&mut host, cb, VIEWPORT);
    assert_eq!(host.node(row).layout.location.y, 5.0);
}
