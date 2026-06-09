//! Solver for idle-games with assumptions:
//! - Income never decreases
//! - Costs depend only on current quantity

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap},
    time::Instant,
};

const GOAL: f64 = 1e12;
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;
const VERBOSE: bool = false;
const EPS: f64 = 1e-9;

#[rustfmt::skip]
struct Resource { name: &'static str, cost_fn: fn(i32) -> f64, yield_fn: fn(i32) -> f64 }

#[rustfmt::skip]
const RESOURCES: [Resource; NUM_RES] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];
type Inv = [i32; NUM_RES];
type StateKey = (i64, Inv);

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
fn inc(inv: &Inv) -> f64 { inv.iter().enumerate().map(|(i, &q)| (RESOURCES[i].yield_fn)(q)).sum() }
#[rustfmt::skip]
fn cost(idx: usize, s: &GameState) -> f64 { (RESOURCES[idx].cost_fn)(s.inventory[idx]) }
#[rustfmt::skip]
fn step(s: GameState, t: i64) -> GameState { GameState { time: s.time + t, money: s.money + s.income * t as f64, ..s } }

fn buy(mut state: GameState, idx: usize) -> Option<GameState> {
    let price = cost(idx, &state);
    (state.money + EPS >= price).then(|| {
        state.money -= price;
        state.inventory[idx] += 1;
        state.income = inc(&state.inventory);
        state
    })
}

fn time_to_money(s: &GameState, goal: f64) -> i64 {
    if s.money >= goal {
        0
    } else if s.income <= 0. || !s.income.is_finite() {
        i64::MAX
    } else {
        let t = ((goal - s.money) / s.income).ceil();
        if !t.is_finite() || t >= i64::MAX as f64 {
            i64::MAX
        } else {
            t as i64
        }
    }
}

fn reconstruct_path(
    mem: &BTreeMap<StateKey, (GameState, usize, StateKey)>,
    mut key: StateKey,
) -> Vec<String> {
    let mut log = Vec::new();
    while let Some(&(s, bought, parent)) = mem.get(&key) {
        if s.time == 0 {
            break;
        }
        log.push(format!(
            "At time {}, bought {}, inventory {:?}, income {:.2}, money {:.2}",
            s.time, RESOURCES[bought].name, s.inventory, s.income, s.money
        ));
        key = parent;
    }
    log.reverse();
    log
}

fn gain(i: usize, s: &GameState) -> f64 {
    let q = s.inventory[i];
    (RESOURCES[i].yield_fn)(q + 1) - (RESOURCES[i].yield_fn)(q)
}

fn roi(i: usize, s: &GameState) -> i64 {
    let c = cost(i, s);
    if c <= 0. {
        i64::MAX
    } else {
        (gain(i, s) / c * 1e12) as i64
    }
}

fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time.saturating_add(time_to_money(s, goal))
}

fn dominates(a: &GameState, b: &GameState) -> bool {
    a.time <= b.time && a.money + a.income * (b.time - a.time) as f64 + EPS >= b.money
}

fn insert_front(front: &mut BTreeMap<Inv, Vec<GameState>>, s: GameState) -> bool {
    let v = front.entry(s.inventory).or_default();

    if v.iter().any(|old| dominates(old, &s)) {
        return false;
    }

    v.retain(|old| !dominates(&s, old));
    v.push(s);
    true
}

fn alive(front: &BTreeMap<Inv, Vec<GameState>>, s: &GameState) -> bool {
    front
        .get(&s.inventory)
        .is_some_and(|v| v.iter().any(|x| x.time == s.time && (x.money - s.money).abs() <= EPS))
}

/*
State heap.

p1 = negative finish time, so earlier projected finish pops first.
p2 = ROI of the move that created this state, only tie-breaker.
*/
#[derive(Clone, Copy)]
struct Node {
    p1: i64,
    p2: i64,
    s: GameState,
}

impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.p1 == o.p1 && self.p2 == o.p2
    }
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        self.p1.cmp(&o.p1).then(self.p2.cmp(&o.p2)).then(o.s.time.cmp(&self.s.time))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

fn push(q: &mut BinaryHeap<Node>, s: GameState, last_roi: i64, goal: f64) {
    q.push(Node { p1: -finish_time(&s, goal), p2: last_roi, s });
}

#[derive(Debug)]
struct SolveResult {
    best_time: i64,
    final_money: f64,
    iterations: usize,
    final_inventory: Inv,
}

fn search(goal: f64, verbose: bool) -> SolveResult {
    let mut mem: BTreeMap<StateKey, (GameState, usize, StateKey)> = BTreeMap::new();
    let s0 = GameState::new();
    mem.insert((0, s0.inventory), (s0, usize::MAX, (0, s0.inventory)));
    let mut best_state = s0;
    let mut best_time = finish_time(&s0, goal);
    let mut pri_q = BinaryHeap::new();
    let mut front: BTreeMap<Inv, Vec<GameState>> = BTreeMap::new();
    insert_front(&mut front, s0);
    push(&mut pri_q, s0, 0, goal);
    let mut iter = 0;

    while let Some(Node { s, .. }) = pri_q.pop() {
        iter += 1;
        if iter >= MAX_NODES {
            break;
        }

        if s.time >= best_time || !alive(&front, &s) {
            continue;
        }

        let wait_finish = finish_time(&s, goal);
        if wait_finish < best_time {
            best_time = wait_finish;
            best_state = s;

            if verbose {
                println!("Iter {iter}: New Best Time Found: {best_time}");
            }
        }

        for i in 0..NUM_RES {
            let wait = time_to_money(&s, cost(i, &s));
            if wait == i64::MAX || s.time + wait >= best_time {
                continue;
            }

            let Some(ns) = buy(step(s, wait), i) else {
                continue;
            };

            let ns_finish = finish_time(&ns, goal);

            // Critical pruning, but now applied only after child construction.
            // This keeps your old useful behavior but avoids decision-node explosion.
            if ns_finish >= wait_finish || ns_finish >= best_time {
                continue;
            }

            if !insert_front(&mut front, ns) {
                continue;
            }

            mem.insert((ns.time, ns.inventory), (ns, i, (s.time, s.inventory)));
            push(&mut pri_q, ns, roi(i, &s), goal);
        }
    }

    if verbose {
        for line in reconstruct_path(&mem, (best_state.time, best_state.inventory)) {
            println!("{line}");
        }
        println!("Then wait until time {best_time} to reach the goal.");
    }
    let final_state = step(best_state, time_to_money(&best_state, goal));
    SolveResult {
        best_time,
        final_money: final_state.money,
        iterations: iter,
        final_inventory: final_state.inventory,
    }
}

fn main() {
    let start = Instant::now();
    let result = search(GOAL, VERBOSE);
    println!("Time elapsed: {:?}", start.elapsed());
    println!("{result:?}");
}
