use rustc_hash::FxHashMap as HashMap;
use std::{cmp::Ordering, collections::BinaryHeap, time::Instant};

#[rustfmt::skip]
struct Resource { name: &'static str, cost_fn: fn(i32) -> f64, yield_fn: fn(i32) -> f64 }
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;

#[rustfmt::skip]
const RESOURCES: [Resource; 3] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];

#[derive(Clone, Copy, Debug)]
struct GameState {
    time: i64,
    money: f64,
    inventory: [i32; NUM_RES],
}
impl GameState {
    fn new() -> Self {
        Self { time: 0, money: 0., inventory: [1, 0, 0] }
    }
}

#[rustfmt::skip]
fn get_inc(s: &GameState) -> f64 { s.inventory.iter().enumerate().map(|(i, &q)| (RESOURCES[i].yield_fn)(q)).sum() }
#[rustfmt::skip]
fn get_cost(idx: usize, s: &GameState) -> f64 { (RESOURCES[idx].cost_fn)(s.inventory[idx]) }
#[rustfmt::skip]
fn step(s: GameState, t: i64) -> GameState { GameState { time: s.time + t, money: s.money + get_inc(&s) * t as f64, ..s } }

fn buy(mut state: GameState, idx: usize) -> Option<GameState> {
    let price = get_cost(idx, &state);
    (state.money >= price).then(|| {
        state.money -= price;
        state.inventory[idx] += 1;
        state
    })
}

fn time_to_money(s: &GameState, goal: f64) -> i64 {
    let inc = get_inc(s);
    if inc <= 0. {
        i64::MAX
    } else {
        ((goal - s.money).max(0.) / inc).ceil() as i64
    }
}

/// Returns (state, time to goal, if slower than just to wait)
fn buy_order(s: GameState, order: usize, goal: f64) -> (GameState, i64, bool) {
    let (time_to_goal, time_to_resource) =
        (time_to_money(&s, goal), time_to_money(&s, get_cost(order, &s)));
    if time_to_goal <= time_to_resource {
        return (s, s.time + time_to_goal, true);
    }
    let s = buy(step(s, time_to_resource), order).unwrap();
    (s, s.time + time_to_money(&s, goal), false)
}

fn reconstruct_path(
    mem: &HashMap<[i32; NUM_RES], (i64, f64, usize)>,
    mut inv: [i32; NUM_RES],
) -> Vec<String> {
    let mut log = Vec::new();
    while let Some(&(t, _, a)) = mem.get(&inv) {
        if a == usize::MAX {
            break;
        }
        log.push(format!("Reached {:?} by buying {} at time {}", inv, RESOURCES[a].name, t));
        inv[a] -= 1;
    }
    log.reverse();
    log
}

// Node implementation (time, state, decision index)
struct Node(i64, GameState, usize);
impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0
    }
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.cmp(&self.0)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[derive(Debug)]
struct SolveResult {
    best_time: i64,
    final_money: f64,
    iterations: usize,
    final_inventory: [i32; NUM_RES],
}

fn search(goal: f64, verbose: bool) -> SolveResult {
    let mut mem = HashMap::with_capacity_and_hasher(100_000, Default::default());
    let s0 = GameState::new();
    mem.insert(s0.inventory, (s0.time, s0.money, usize::MAX));

    let (mut best_time, mut best_g) = (s0.time + time_to_money(&s0, goal), s0);
    let mut pri_q: BinaryHeap<_> = (0..NUM_RES).map(|i| Node(s0.time, s0, i)).collect();
    let mut iter = 0;

    while let Some(Node(time, current_state, order)) = pri_q.pop() {
        iter += 1;
        if iter >= MAX_NODES {
            break;
        }
        // if we are already later than best time
        if current_state.time >= best_time {
            continue;
        }
        let (next_state, next_complete_time, done) = buy_order(current_state, order, goal);
        // if the purchase made us worse off
        if next_complete_time >= current_state.time + time_to_money(&current_state, goal) {
            continue;
        }

        // check memory
        if mem.get(&next_state.inventory).map_or(true, |&(mt, mm, _)| {
            next_state.time < mt || (next_state.time == mt && next_state.money > mm)
        }) {
            mem.insert(next_state.inventory, (next_state.time, next_state.money, order));
            if !done && next_state.time < best_time {
                pri_q.extend((0..NUM_RES).map(|i| Node(next_state.time, next_state, i)));
            }
        }
        // if better we update
        if next_complete_time < best_time {
            best_time = next_complete_time;
            best_g = next_state;
            if verbose {
                println!("Iter: {iter}: New Best Time Found: {best_time}");
            }
        }
    }

    best_g = step(best_g, time_to_money(&best_g, goal));
    if verbose {
        for line in reconstruct_path(&mem, best_g.inventory) {
            println!("{line}");
        }
    }
    SolveResult {
        best_time,
        final_money: best_g.money,
        iterations: iter,
        final_inventory: best_g.inventory,
    }
}

fn main() {
    let start = Instant::now();
    let result = search(1e12, false);
    println!("Time elapsed: {:?}", start.elapsed());
    println!("{result:?}");
}
