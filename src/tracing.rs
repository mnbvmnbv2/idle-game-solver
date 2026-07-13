use serde::Serialize;
use std::{
    fs::File,
    io::{BufWriter, Result as IoResult},
};

use crate::{
    game::{GameRules, GameState, Inventory},
    objective::Objective,
    solver::SolveResult,
};

const TRACE_MAX_NODES: usize = 250_000;
const TRACE_MAX_EVENTS: usize = 1_000_000;
const TRACE_POP_STRIDE: usize = 25;

#[derive(Clone, Debug)]
pub struct AcceptedNode {
    pub parent: Option<usize>,
    pub bought: Option<usize>,
    pub iter_created: usize,
    pub state: GameState,
    pub finish_time: i64,
    pub wait: Option<i64>,
    pub cost_paid: Option<f64>,
}

pub trait SearchObserver {
    fn accept_node(&mut self, node: AcceptedNode) -> usize;
    fn start(&mut self, _node: usize, _best_time: i64) {}
    fn pop(&mut self, _node: usize, _iter: usize, _queue_len: usize, _best_time: i64) {}
    fn accept_buy(
        &mut self,
        _node: usize,
        _parent: usize,
        _iter: usize,
        _bought: usize,
        _finish_time: i64,
    ) {
    }
    fn reject_buy(&mut self, _parent: usize, _iter: usize, _bought: usize, _reason: &'static str) {}
    fn best(&mut self, _node: usize, _iter: usize, _best_time: i64) {}
    fn prune(&mut self, _node: usize, _iter: usize, _reason: &'static str) {}
    fn truncated(&mut self, _iter: usize, _what: &'static str) {}
    fn finish(&mut self, _best_node: usize) {}
}

#[derive(Default)]
pub struct NullTrace {
    next_id: usize,
}

impl SearchObserver for NullTrace {
    fn accept_node(&mut self, _node: AcceptedNode) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[derive(Serialize)]
struct TraceResource {
    name: String,
}

#[derive(Serialize)]
struct TraceMeta {
    game: String,
    objective: String,
    trace_max_nodes: usize,
    trace_max_events: usize,
    trace_pop_stride: usize,
    iterations: usize,
    best_time: i64,
    final_money: f64,
    final_inventory: Inventory,
    truncated_nodes: bool,
    truncated_events: bool,
}

#[derive(Serialize)]
struct TraceNode {
    id: usize,
    parent: Option<usize>,
    bought: Option<usize>,
    bought_name: Option<String>,
    depth: usize,
    iter_created: usize,

    time: i64,
    money: f64,
    inventory: Inventory,
    income: f64,
    finish_time: i64,

    wait: Option<i64>,
    cost_paid: Option<f64>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum TraceEvent {
    Start { node: usize, iter: usize, best_time: i64 },
    Pop { node: usize, iter: usize, queue_len: usize, best_time: i64 },
    Accept { node: usize, parent: usize, iter: usize, bought: usize, finish_time: i64 },
    Best { node: usize, iter: usize, best_time: i64 },
    Prune { node: usize, iter: usize, reason: &'static str },
    RejectBuy { parent: usize, iter: usize, bought: usize, reason: &'static str },
    Truncated { iter: usize, what: &'static str },
}

#[derive(Serialize)]
struct TraceFile {
    meta: TraceMeta,
    resources: Vec<TraceResource>,
    nodes: Vec<TraceNode>,
    events: Vec<TraceEvent>,
    best_path_node_ids: Vec<usize>,
}

pub struct JsonTrace {
    game_name: String,
    objective_label: String,
    resource_names: Vec<String>,
    next_id: usize,
    nodes: Vec<TraceNode>,
    events: Vec<TraceEvent>,
    depth_by_id: Vec<usize>,
    parent_by_id: Vec<Option<usize>>,
    best_node_id: usize,
    truncated_nodes: bool,
    truncated_events: bool,
}

impl JsonTrace {
    pub fn new(rules: &GameRules, objective: &Objective) -> Self {
        Self {
            game_name: rules.name.clone(),
            objective_label: objective.label(),
            resource_names: rules.resources.iter().map(|r| r.name.clone()).collect(),
            next_id: 0,
            nodes: Vec::new(),
            events: Vec::new(),
            depth_by_id: Vec::new(),
            parent_by_id: Vec::new(),
            best_node_id: 0,
            truncated_nodes: false,
            truncated_events: false,
        }
    }

    pub fn write(self, path: &str, result: &SolveResult) -> IoResult<()> {
        let best_path_node_ids = self.best_path(self.best_node_id);

        let resources =
            self.resource_names.iter().map(|name| TraceResource { name: name.clone() }).collect();

        let file = TraceFile {
            meta: TraceMeta {
                game: self.game_name,
                objective: self.objective_label,
                trace_max_nodes: TRACE_MAX_NODES,
                trace_max_events: TRACE_MAX_EVENTS,
                trace_pop_stride: TRACE_POP_STRIDE,
                iterations: result.iterations,
                best_time: result.best_time,
                final_money: result.final_money,
                final_inventory: result.final_inventory.clone(),
                truncated_nodes: self.truncated_nodes,
                truncated_events: self.truncated_events,
            },
            resources,
            nodes: self.nodes,
            events: self.events,
            best_path_node_ids,
        };

        let out = File::create(path)?;
        let writer = BufWriter::new(out);
        serde_json::to_writer_pretty(writer, &file)?;

        Ok(())
    }

    fn push_event(&mut self, event: TraceEvent) {
        if self.events.len() < TRACE_MAX_EVENTS {
            self.events.push(event);
        } else {
            self.truncated_events = true;
        }
    }

    fn best_path(&self, mut id: usize) -> Vec<usize> {
        let mut path = Vec::new();

        loop {
            path.push(id);

            let Some(Some(parent)) = self.parent_by_id.get(id) else {
                break;
            };

            id = *parent;
        }

        path.reverse();
        path
    }
}

impl SearchObserver for JsonTrace {
    fn accept_node(&mut self, node: AcceptedNode) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let depth = node.parent.and_then(|p| self.depth_by_id.get(p).copied()).map_or(0, |d| d + 1);

        self.depth_by_id.push(depth);
        self.parent_by_id.push(node.parent);

        if self.nodes.len() < TRACE_MAX_NODES {
            self.nodes.push(TraceNode {
                id,
                parent: node.parent,
                bought: node.bought,
                bought_name: node.bought.and_then(|i| self.resource_names.get(i).cloned()),
                depth,
                iter_created: node.iter_created,

                time: node.state.time,
                money: node.state.money,
                inventory: node.state.inventory,
                income: node.state.income,
                finish_time: node.finish_time,

                wait: node.wait,
                cost_paid: node.cost_paid,
            });
        } else {
            self.truncated_nodes = true;
        }

        id
    }

    fn start(&mut self, node: usize, best_time: i64) {
        self.push_event(TraceEvent::Start { node, iter: 0, best_time });
    }

    fn pop(&mut self, node: usize, iter: usize, queue_len: usize, best_time: i64) {
        if iter % TRACE_POP_STRIDE == 0 {
            self.push_event(TraceEvent::Pop { node, iter, queue_len, best_time });
        }
    }

    fn accept_buy(
        &mut self,
        node: usize,
        parent: usize,
        iter: usize,
        bought: usize,
        finish_time: i64,
    ) {
        self.push_event(TraceEvent::Accept { node, parent, iter, bought, finish_time });
    }

    fn reject_buy(&mut self, parent: usize, iter: usize, bought: usize, reason: &'static str) {
        // Rejected branches can be extremely noisy.
        // Leave this disabled unless you specifically want rejection visualization.
        self.push_event(TraceEvent::RejectBuy { parent, iter, bought, reason });
    }

    fn best(&mut self, node: usize, iter: usize, best_time: i64) {
        self.best_node_id = node;

        self.push_event(TraceEvent::Best { node, iter, best_time });
    }

    fn prune(&mut self, node: usize, iter: usize, reason: &'static str) {
        self.push_event(TraceEvent::Prune { node, iter, reason });
    }

    fn truncated(&mut self, iter: usize, what: &'static str) {
        self.push_event(TraceEvent::Truncated { iter, what });
    }

    fn finish(&mut self, best_node: usize) {
        self.best_node_id = best_node;
    }
}
