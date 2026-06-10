mod tracing;

use std::{cmp::Ordering, collections::BinaryHeap, env, time::Instant};

use tracing::{AcceptedNode, JsonTrace, NullTrace, SearchObserver};

const DEFAULT_GOAL: f64 = 1e12;
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;
const VERBOSE: bool = false;

#[rustfmt::skip]
pub struct Resource {
    pub name: &'static str,
    pub cost_fn: fn(i32) -> f64,
    pub yield_fn: fn(i32) -> f64,
}

#[rustfmt::skip]
pub const RESOURCES: [Resource; NUM_RES] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];

pub type Inv = [i32; NUM_RES];

#[derive(Clone, Copy, Debug)]
pub struct GameState {
    pub time: i64,
    pub money: f64,
    pub inventory: Inv,
    pub income: f64,
}

#[derive(Debug)]
pub struct SolveResult {
    pub best_time: i64,
    pub final_money: f64,
    pub iterations: usize,
    pub final_inventory: Inv,
}

#[derive(Clone, Copy, Debug)]
struct BuyResult {
    state: GameState,
    finish_time: i64,
    wait: i64,
    cost_paid: f64,
}

#[derive(Debug)]
struct GameData {
    max_inventory: Inv,
    cost: Vec<Vec<f64>>,
    yield_: Vec<Vec<f64>>,
    delta: Vec<Vec<f64>>,
}

impl GameData {
    fn new(goal: f64) -> Self {
        let mut max_inventory = [1, 0, 0];
        let (mut cost, mut yield_, mut delta) = (Vec::new(), Vec::new(), Vec::new());

        for (i, res) in RESOURCES.iter().enumerate() {
            while (res.cost_fn)(max_inventory[i]) < goal {
                max_inventory[i] += 1;
                assert!(max_inventory[i] <= 1_000_000, "Resource {} has no cap", res.name);
            }
            let max_q = max_inventory[i];
            let ys: Vec<f64> = (0..=max_q + 1).map(|q| (res.yield_fn)(q)).collect();
            let cs: Vec<f64> = (0..=max_q).map(|q| (res.cost_fn)(q)).collect();
            let ds: Vec<f64> = (0..=max_q).map(|q| ys[q as usize + 1] - ys[q as usize]).collect();

            cost.push(cs);
            yield_.push(ys);
            delta.push(ds);
        }

        Self { max_inventory, cost, yield_, delta }
    }

    #[inline]
    fn can_buy_more(&self, inv: &Inv, i: usize) -> bool {
        inv[i] < self.max_inventory[i]
    }
    #[inline]
    fn cost(&self, i: usize, q: i32) -> f64 {
        self.cost[i][q as usize]
    }
    #[inline]
    fn delta(&self, i: usize, q: i32) -> f64 {
        self.delta[i][q as usize]
    }
    fn income(&self, inv: &Inv) -> f64 {
        inv.iter().enumerate().map(|(i, &q)| self.yield_[i][q as usize]).sum()
    }
    fn initial_state(&self) -> GameState {
        let inventory = [1, 0, 0];
        GameState { time: 0, money: 0., inventory, income: self.income(&inventory) }
    }
}
#[derive(Clone, Copy, Debug)]
struct MemoEntry {
    time: i64,
    money: f64,
    bought: usize,
    node_id: usize,
}
impl MemoEntry {
    const EMPTY: Self =
        Self { time: i64::MAX, money: f64::NEG_INFINITY, bought: usize::MAX, node_id: usize::MAX };
    #[inline]
    fn is_empty(self) -> bool {
        self.time == i64::MAX
    }
}
#[derive(Debug)]
struct MemoTable {
    dims: [usize; NUM_RES],
    data: Vec<MemoEntry>,
}
impl MemoTable {
    fn new(max: Inv) -> Self {
        let dims = [max[0] as usize + 1, max[1] as usize + 1, max[2] as usize + 1];
        Self { dims, data: vec![MemoEntry::EMPTY; dims.iter().product()] }
    }
    #[inline]
    fn idx(&self, inv: &Inv) -> usize {
        (inv[0] as usize * self.dims[1] + inv[1] as usize) * self.dims[2] + inv[2] as usize
    }
    #[inline]
    fn get(&self, inv: &Inv) -> Option<MemoEntry> {
        let m = self.data[self.idx(inv)];
        (!m.is_empty()).then_some(m)
    }
    #[inline]
    fn insert(&mut self, inv: Inv, entry: MemoEntry) {
        let idx = self.idx(&inv);
        self.data[idx] = entry;
    }
    #[inline]
    fn is_better(&self, inv: &Inv, time: i64, money: f64) -> bool {
        let m = self.data[self.idx(inv)];
        m.is_empty() || time < m.time || (time == m.time && money > m.money)
    }
}

#[inline]
fn step(s: &GameState, t: i64) -> GameState {
    GameState { time: s.time + t, money: s.money + s.income * t as f64, ..*s }
}
#[inline]
fn time_to_money(s: &GameState, goal: f64) -> i64 {
    if s.money >= goal {
        0
    } else if s.income <= 0. {
        i64::MAX
    } else {
        ((goal - s.money) / s.income).ceil() as i64
    }
}
#[inline]
fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time.saturating_add(time_to_money(s, goal))
}

#[inline]
fn buy_next(
    data: &GameData,
    s: &GameState,
    i: usize,
    goal: f64,
    c: f64,
    wait: i64,
) -> Option<BuyResult> {
    if wait == i64::MAX || s.time + wait >= finish_time(s, goal) {
        return None;
    }
    let mut ns = step(s, wait);
    ns.money -= c;
    ns.inventory[i] += 1;
    ns.income += data.delta(i, s.inventory[i]);
    Some(BuyResult { finish_time: finish_time(&ns, goal), state: ns, wait, cost_paid: c })
}

#[inline]
fn next_buy_is_order_dominated(
    candidate: usize,
    costs: &[f64; NUM_RES],
    waits: &[i64; NUM_RES],
    deltas: &[f64; NUM_RES],
) -> bool {
    let wait_c = waits[candidate];
    wait_c != i64::MAX
        && (0..NUM_RES).any(|first| {
            first != candidate
                && waits[first] < wait_c
                && deltas[first] > 0.
                && deltas[first] * (wait_c - waits[first]) as f64 + 1e-9 >= costs[first]
        })
}

fn reconstruct_path(mem: &MemoTable, mut inv: Inv) -> Vec<String> {
    let mut log = Vec::new();
    while let Some(m) = mem.get(&inv).filter(|m| m.bought != usize::MAX) {
        log.push(format!(
            "At time {}, bought {}, inventory {:?}",
            m.time, RESOURCES[m.bought].name, inv
        ));
        inv[m.bought] -= 1;
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
        other.priority.cmp(&self.priority)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn search<O: SearchObserver>(goal: f64, verbose: bool, observer: &mut O) -> SolveResult {
    let data = GameData::new(goal);
    let mut mem = MemoTable::new(data.max_inventory);
    let mut q = BinaryHeap::new();
    let s0 = data.initial_state();
    let s0_finish = finish_time(&s0, goal);

    let root_id = observer.accept_node(AcceptedNode {
        parent: None,
        bought: None,
        iter_created: 0,
        state: s0,
        finish_time: s0_finish,
        wait: None,
        cost_paid: None,
    });

    observer.start(root_id, s0_finish);
    mem.insert(
        s0.inventory,
        MemoEntry { time: 0, money: 0., bought: usize::MAX, node_id: root_id },
    );
    q.push(Node { priority: s0.time, state: s0, id: root_id });

    let mut best_time = s0_finish;
    let mut best_game = s0;
    let mut best_node_id = root_id;
    let mut iter = 0;

    while let Some(Node { state: s, id, .. }) = q.pop() {
        iter += 1;

        if iter >= MAX_NODES {
            observer.truncated(iter, "max_nodes_reached");
            break;
        }

        observer.pop(id, iter, q.len(), best_time);

        if s.time >= best_time {
            observer.prune(id, iter, "state_time_not_better_than_best");
            continue;
        }

        if let Some(m) = mem.get(&s.inventory) {
            if id != m.node_id && (s.time > m.time || (s.time == m.time && s.money < m.money)) {
                observer.prune(id, iter, "lazy_deleted_dominated_inventory");
                continue;
            }
        }

        let current_finish = finish_time(&s, goal);
        let mut costs = [0.0; NUM_RES];
        let mut waits = [i64::MAX; NUM_RES];
        let mut deltas = [0.0; NUM_RES];

        for i in 0..NUM_RES {
            if data.can_buy_more(&s.inventory, i) {
                costs[i] = data.cost(i, s.inventory[i]);
                waits[i] = time_to_money(&s, costs[i]);
                deltas[i] = data.delta(i, s.inventory[i]);
            }
        }

        for i in 0..NUM_RES {
            if next_buy_is_order_dominated(i, &costs, &waits, &deltas) {
                observer.reject_buy(id, iter, i, "next_buy_order_dominated");
                continue;
            }

            let Some(buy) = buy_next(&data, &s, i, goal, costs[i], waits[i]) else {
                observer.reject_buy(id, iter, i, "cannot_buy_before_current_finish");
                continue;
            };

            if buy.finish_time >= current_finish {
                observer.reject_buy(id, iter, i, "does_not_improve_parent_finish");
                continue;
            }

            if buy.state.time >= best_time {
                observer.reject_buy(id, iter, i, "buy_time_after_best");
                continue;
            }

            if !mem.is_better(&buy.state.inventory, buy.state.time, buy.state.money) {
                observer.reject_buy(id, iter, i, "dominated_inventory");
                continue;
            }

            let child_id = observer.accept_node(AcceptedNode {
                parent: Some(id),
                bought: Some(i),
                iter_created: iter,
                state: buy.state,
                finish_time: buy.finish_time,
                wait: Some(buy.wait),
                cost_paid: Some(buy.cost_paid),
            });

            observer.accept_buy(child_id, id, iter, i, buy.finish_time);
            mem.insert(
                buy.state.inventory,
                MemoEntry {
                    time: buy.state.time,
                    money: buy.state.money,
                    bought: i,
                    node_id: child_id,
                },
            );

            if buy.finish_time < best_time {
                best_time = buy.finish_time;
                best_game = buy.state;
                best_node_id = child_id;

                observer.best(child_id, iter, best_time);
                if verbose {
                    println!("Iter {iter}: New Best Time Found: {best_time}");
                }
            }

            q.push(Node { priority: buy.state.time, state: buy.state, id: child_id });
        }
    }

    if verbose {
        for line in reconstruct_path(&mem, best_game.inventory) {
            println!("{line}");
        }
        println!("Then wait until time {best_time} to reach the goal.");
    }

    let final_state = step(&best_game, time_to_money(&best_game, goal));
    observer.finish(best_node_id);

    SolveResult {
        best_time,
        final_money: final_state.money,
        iterations: iter,
        final_inventory: final_state.inventory,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let goal = args.get(1).and_then(|s| s.parse::<f64>().ok()).unwrap_or(DEFAULT_GOAL);
    let trace_path = args.get(2).map(String::as_str);

    let start = Instant::now();

    if let Some(path) = trace_path {
        let mut trace = JsonTrace::new();
        let result = search(goal, VERBOSE, &mut trace);
        println!("Time elapsed: {:?}\n{result:?}", start.elapsed());

        match trace.write(path, goal, &result) {
            Ok(()) => println!("Wrote trace to {path}"),
            Err(e) => eprintln!("Failed to write trace: {e}"),
        }
    } else {
        let mut trace = NullTrace::default();
        let result = search(goal, VERBOSE, &mut trace);
        println!("Time elapsed: {:?}\n{result:?}", start.elapsed());
    }
}
