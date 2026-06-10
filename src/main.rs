//! Solver for idle-games with assumptions:
//! - Income never decreases
//! - Costs depend only on current quantity

use rustc_hash::FxHashMap as HashMap;
use serde::Serialize;
use std::{cmp::Ordering, collections::BinaryHeap, env, fs::File, io::BufWriter, time::Instant};

const GOAL: f64 = 1e12;
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;
const VERBOSE: bool = false;

// Trace controls.
// For very large searches, keep this capped.
// Set TRACE_MAX_NODES to usize::MAX for full trace, but beware huge JSON.
const TRACE_ENABLED: bool = true;
const TRACE_MAX_NODES: usize = 250_000;
const TRACE_MAX_EVENTS: usize = 1_000_000;

// Emit every Nth pop event. Accepted nodes are still emitted until TRACE_MAX_NODES.
const TRACE_POP_STRIDE: usize = 25;

#[rustfmt::skip]
struct Resource {
    name: &'static str,
    cost_fn: fn(i32) -> f64,
    yield_fn: fn(i32) -> f64,
}

#[rustfmt::skip]
const RESOURCES: [Resource; NUM_RES] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];

type Inv = [i32; NUM_RES];

#[derive(Clone, Copy, Debug)]
struct GameState {
    time: i64,
    money: f64,
    inventory: Inv,
    income: f64,
}

impl GameState {
    fn new() -> Self {
        let inventory = [1, 0, 0];
        Self { time: 0, money: 0., inventory, income: inc(&inventory) }
    }
}

#[rustfmt::skip]
fn inc(inv: &Inv) -> f64 {
    inv.iter()
        .enumerate()
        .map(|(i, &q)| (RESOURCES[i].yield_fn)(q))
        .sum()
}

#[rustfmt::skip]
fn cost(i: usize, inv: &Inv) -> f64 {
    (RESOURCES[i].cost_fn)(inv[i])
}

#[rustfmt::skip]
fn step(s: &GameState, t: i64) -> GameState {
    GameState {
        time: s.time + t,
        money: s.money + s.income * t as f64,
        ..*s
    }
}

fn time_to_money(s: &GameState, goal: f64) -> i64 {
    if s.money >= goal {
        0
    } else if s.income <= 0. {
        i64::MAX
    } else {
        ((goal - s.money) / s.income).ceil() as i64
    }
}

fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time.saturating_add(time_to_money(s, goal))
}

fn buy_next(s: &GameState, i: usize, goal: f64) -> Option<(GameState, i64, i64, f64)> {
    let c = cost(i, &s.inventory);
    let wait = time_to_money(s, c);

    if wait == i64::MAX || s.time + wait >= finish_time(s, goal) {
        return None;
    }

    let mut ns = step(s, wait);
    ns.money -= cost(i, &ns.inventory);
    ns.inventory[i] += 1;
    ns.income = inc(&ns.inventory);

    let ns_finish = finish_time(&ns, goal);
    Some((ns, ns_finish, wait, c))
}

fn reconstruct_path(mem: &HashMap<Inv, (i64, f64, usize, usize)>, mut inv: Inv) -> Vec<String> {
    let mut log = Vec::new();

    while let Some(&(t, _, bought, _node_id)) = mem.get(&inv) {
        if bought == usize::MAX {
            break;
        }

        log.push(format!("At time {}, bought {}, inventory {:?}", t, RESOURCES[bought].name, inv));

        inv[bought] -= 1;
    }

    log.reverse();
    log
}

#[derive(Clone)]
struct Node {
    priority: i64,
    state: GameState,
    id: usize,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for Node {}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering: BinaryHeap is max-heap by default.
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug)]
struct SolveResult {
    best_time: i64,
    final_money: f64,
    iterations: usize,
    final_inventory: Inv,
}

#[derive(Serialize)]
struct TraceResource {
    name: &'static str,
}

#[derive(Serialize)]
struct TraceMeta {
    goal: f64,
    max_nodes: usize,
    trace_max_nodes: usize,
    trace_max_events: usize,
    trace_pop_stride: usize,
    iterations: usize,
    best_time: i64,
    final_money: f64,
    final_inventory: Inv,
    truncated_nodes: bool,
    truncated_events: bool,
}

#[derive(Serialize)]
struct TraceNode {
    id: usize,
    parent: Option<usize>,
    bought: Option<usize>,
    bought_name: Option<&'static str>,
    depth: usize,
    iter_created: usize,

    time: i64,
    money: f64,
    inventory: Inv,
    income: f64,
    finish_time: i64,

    wait: Option<i64>,
    cost_paid: Option<f64>,
}

#[derive(Serialize, Copy, Clone)]
#[serde(tag = "type")]
enum TraceEvent {
    Start { node: usize, iter: usize, best_time: i64 },
    Pop { node: usize, iter: usize, queue_len: usize, best_time: i64 },
    Accept { node: usize, parent: usize, iter: usize, bought: usize, finish_time: i64 },
    Best { node: usize, iter: usize, best_time: i64 },
    Prune { node: usize, iter: usize, reason: &'static str },
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

struct TraceCollector {
    enabled: bool,
    nodes: Vec<TraceNode>,
    events: Vec<TraceEvent>,
    depth_by_id: Vec<usize>,
    parent_by_id: Vec<Option<usize>>,
    truncated_nodes: bool,
    truncated_events: bool,
}

impl TraceCollector {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            nodes: Vec::new(),
            events: Vec::new(),
            depth_by_id: Vec::new(),
            parent_by_id: Vec::new(),
            truncated_nodes: false,
            truncated_events: false,
        }
    }

    fn can_emit_node(&self) -> bool {
        self.enabled && self.nodes.len() < TRACE_MAX_NODES
    }

    fn push_event(&mut self, event: TraceEvent) {
        if !self.enabled {
            return;
        }

        if self.events.len() < TRACE_MAX_EVENTS {
            self.events.push(event);
        } else {
            self.truncated_events = true;
        }
    }

    fn add_node(
        &mut self,
        parent: Option<usize>,
        bought: Option<usize>,
        iter_created: usize,
        state: &GameState,
        finish_time: i64,
        wait: Option<i64>,
        cost_paid: Option<f64>,
    ) -> usize {
        let id = self.depth_by_id.len();
        let depth = parent.and_then(|p| self.depth_by_id.get(p).copied()).map_or(0, |d| d + 1);

        self.depth_by_id.push(depth);
        self.parent_by_id.push(parent);

        if self.can_emit_node() {
            self.nodes.push(TraceNode {
                id,
                parent,
                bought,
                bought_name: bought.map(|i| RESOURCES[i].name),
                depth,
                iter_created,
                time: state.time,
                money: state.money,
                inventory: state.inventory,
                income: state.income,
                finish_time,
                wait,
                cost_paid,
            });
        } else if self.enabled {
            self.truncated_nodes = true;
        }

        id
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

    fn write(
        self,
        path: &str,
        goal: f64,
        result: &SolveResult,
        best_node_id: usize,
        final_money: f64,
    ) -> std::io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let best_path_node_ids = self.best_path(best_node_id);

        let resources = RESOURCES.iter().map(|r| TraceResource { name: r.name }).collect();

        let file = TraceFile {
            meta: TraceMeta {
                goal,
                max_nodes: MAX_NODES,
                trace_max_nodes: TRACE_MAX_NODES,
                trace_max_events: TRACE_MAX_EVENTS,
                trace_pop_stride: TRACE_POP_STRIDE,
                iterations: result.iterations,
                best_time: result.best_time,
                final_money,
                final_inventory: result.final_inventory,
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
}

fn search(goal: f64, verbose: bool, trace: &mut TraceCollector) -> (SolveResult, usize) {
    let mut mem: HashMap<Inv, (i64, f64, usize, usize)> =
        HashMap::with_capacity_and_hasher(100_000, Default::default());

    let mut q = BinaryHeap::new();

    let s0 = GameState::new();
    let s0_finish = finish_time(&s0, goal);

    let root_id = trace.add_node(None, None, 0, &s0, s0_finish, None, None);

    mem.insert(s0.inventory, (0, 0., usize::MAX, root_id));

    q.push(Node { priority: s0.time, state: s0, id: root_id });

    let mut best_time = s0_finish;
    let mut best_game = s0;
    let mut best_node_id = root_id;
    let mut iter = 0;

    trace.push_event(TraceEvent::Start { node: root_id, iter, best_time });

    while let Some(Node { state: s, id, .. }) = q.pop() {
        iter += 1;

        if iter >= MAX_NODES {
            trace.push_event(TraceEvent::Truncated { iter, what: "max_nodes_reached" });
            break;
        }

        if iter % TRACE_POP_STRIDE == 0 {
            trace.push_event(TraceEvent::Pop { node: id, iter, queue_len: q.len(), best_time });
        }

        if s.time >= best_time {
            trace.push_event(TraceEvent::Prune {
                node: id,
                iter,
                reason: "state_time_not_better_than_best",
            });
            continue;
        }

        // Lazy deletion:
        // If a faster path reached this inventory while this node waited in the queue, drop it.
        if let Some(&(mt, mm, _, best_mem_id)) = mem.get(&s.inventory) {
            if id != best_mem_id && (s.time > mt || (s.time == mt && s.money < mm)) {
                trace.push_event(TraceEvent::Prune {
                    node: id,
                    iter,
                    reason: "lazy_deleted_dominated_inventory",
                });
                continue;
            }
        }

        for i in 0..NUM_RES {
            let Some((ns, ns_finish, wait, cost_paid)) = buy_next(&s, i, goal) else {
                continue;
            };

            if ns_finish >= finish_time(&s, goal) || ns.time >= best_time {
                continue;
            }

            let is_better = mem
                .get(&ns.inventory)
                .map_or(true, |&(mt, mm, _, _)| ns.time < mt || (ns.time == mt && ns.money > mm));

            if is_better {
                let child_id = trace.add_node(
                    Some(id),
                    Some(i),
                    iter,
                    &ns,
                    ns_finish,
                    Some(wait),
                    Some(cost_paid),
                );

                mem.insert(ns.inventory, (ns.time, ns.money, i, child_id));

                trace.push_event(TraceEvent::Accept {
                    node: child_id,
                    parent: id,
                    iter,
                    bought: i,
                    finish_time: ns_finish,
                });

                if ns_finish < best_time {
                    best_time = ns_finish;
                    best_game = ns;
                    best_node_id = child_id;

                    trace.push_event(TraceEvent::Best { node: child_id, iter, best_time });

                    if verbose {
                        println!("Iter {iter}: New Best Time Found: {best_time}");
                    }
                }

                q.push(Node { priority: ns.time, state: ns, id: child_id });
            }
        }
    }

    if verbose {
        for line in reconstruct_path(&mem, best_game.inventory) {
            println!("{line}");
        }
        println!("Then wait until time {best_time} to reach the goal.");
    }

    let final_state = step(&best_game, time_to_money(&best_game, goal));

    (
        SolveResult {
            best_time,
            final_money: final_state.money,
            iterations: iter,
            final_inventory: final_state.inventory,
        },
        best_node_id,
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let goal = args.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(GOAL);

    let trace_path = args.get(2).map(String::as_str).unwrap_or("trace.json");

    let start = Instant::now();

    let mut trace = TraceCollector::new(TRACE_ENABLED);
    let (result, best_node_id) = search(goal, VERBOSE, &mut trace);

    println!("Time elapsed: {:?}", start.elapsed());
    println!("{result:?}");

    if TRACE_ENABLED {
        match trace.write(trace_path, goal, &result, best_node_id, result.final_money) {
            Ok(()) => println!("Wrote trace to {trace_path}"),
            Err(e) => eprintln!("Failed to write trace: {e}"),
        }
    }
}
