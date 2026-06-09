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
const VERBOSE: bool = true;
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

#[derive(Clone, Debug)]
struct GameState {
    id: usize,
    parent_id: usize,
    last_buy: usize,
    time: i64,
    money: f64,
    inventory: Inv,
    income: f64,
}

#[rustfmt::skip]
fn inc(inv: &Inv) -> f64 { inv.iter().enumerate().map(|(i, &q)| (RESOURCES[i].yield_fn)(q)).sum()}
#[rustfmt::skip]
fn cost(idx: usize, inv: &Inv) -> f64 {(RESOURCES[idx].cost_fn)(inv[idx])}
#[rustfmt::skip]
fn step(s: &GameState, t: i64) -> GameState {GameState { time: s.time + t, money: s.money + s.income * t as f64, ..s.clone() }
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

fn gain(i: usize, inv: &Inv) -> f64 {
    (RESOURCES[i].yield_fn)(inv[i] + 1) - (RESOURCES[i].yield_fn)(inv[i])
}

fn roi(i: usize, inv: &Inv) -> i64 {
    let c = cost(i, inv);
    if c <= 0. {
        i64::MAX
    } else {
        (gain(i, inv) / c * 1e12) as i64
    }
}

fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time.saturating_add(time_to_money(s, goal))
}

fn dominates(a: &GameState, b: &GameState) -> bool {
    a.time <= b.time && a.money + a.income * (b.time - a.time) as f64 + EPS >= b.money
}

#[derive(Clone, Copy)]
struct Node {
    p1: i64,
    p2: i64,
    id: usize,
}

impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        self.p1 == o.p1 && self.p2 == o.p2
    }
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        self.p1.cmp(&o.p1).then(self.p2.cmp(&o.p2))
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
    final_inventory: Inv,
}

fn search(goal: f64, verbose: bool) -> SolveResult {
    let mut history: Vec<GameState> = Vec::new();
    let mut front: BTreeMap<Inv, Vec<usize>> = BTreeMap::new();
    let mut pri_q = BinaryHeap::new();

    let s0 = GameState {
        id: 0,
        parent_id: 0,
        last_buy: usize::MAX,
        time: 0,
        money: 0.,
        inventory: [1, 0, 0],
        income: inc(&[1, 0, 0]),
    };

    history.push(s0.clone());
    front.entry(s0.inventory).or_default().push(s0.id);
    pri_q.push(Node { p1: -finish_time(&s0, goal), p2: 0, id: s0.id });
    let mut best_state_id = 0;
    let mut best_time = finish_time(&s0, goal);
    let mut iter = 0;

    while let Some(Node { id, .. }) = pri_q.pop() {
        iter += 1;
        if iter >= MAX_NODES {
            break;
        }

        let s = history[id].clone();
        if s.time >= best_time {
            continue;
        }
        let still_alive = front.get(&s.inventory).is_some_and(|v| v.contains(&id));
        if !still_alive {
            continue;
        }

        let wait_finish = finish_time(&s, goal);
        if wait_finish < best_time {
            best_time = wait_finish;
            best_state_id = id;
            if verbose {
                println!("Iter {iter}: New Best Time Found: {best_time}");
            }
        }

        for i in 0..NUM_RES {
            let wait = time_to_money(&s, cost(i, &s.inventory));
            if wait == i64::MAX || s.time + wait >= best_time {
                continue;
            }

            let mut ns = step(&s, wait);
            ns.money -= cost(i, &ns.inventory);
            ns.inventory[i] += 1;
            ns.income = inc(&ns.inventory);

            ns.id = history.len();
            ns.parent_id = s.id;
            ns.last_buy = i;

            let ns_finish = finish_time(&ns, goal);
            if ns_finish >= wait_finish || ns_finish >= best_time {
                continue;
            }

            let v = front.entry(ns.inventory).or_default();
            if v.iter().any(|&old_id| dominates(&history[old_id], &ns)) {
                continue;
            }
            v.retain(|&old_id| !dominates(&ns, &history[old_id]));
            v.push(ns.id);

            history.push(ns.clone());
            pri_q.push(Node { p1: -finish_time(&ns, goal), p2: roi(i, &s.inventory), id: ns.id });
        }
    }

    if verbose {
        let mut log = Vec::new();
        let mut curr = best_state_id;
        while curr != 0 {
            let s = &history[curr];
            log.push(format!(
                "At time {}, bought {}, inventory {:?}, income {:.2}, money {:.2}",
                s.time, RESOURCES[s.last_buy].name, s.inventory, s.income, s.money
            ));
            curr = s.parent_id;
        }
        log.reverse();
        for line in log {
            println!("{line}");
        }
        println!("Then wait until time {best_time} to reach the goal.");
    }
    let best_state = &history[best_state_id];
    let final_state = step(best_state, time_to_money(best_state, goal));
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
