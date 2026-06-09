//! Solver for idle-games with assumptions:
//! - Income never decreases
//! - Costs depend only on current quantity

use rustc_hash::FxHashMap as HashMap;
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    time::Instant,
};

const GOAL: f64 = 1e12;
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;
const VERBOSE: bool = false;

#[rustfmt::skip]
struct Resource { name: &'static str, cost_fn: fn(i32) -> f64, yield_fn: fn(i32) -> f64 }

#[rustfmt::skip]
const RESOURCES: [Resource; NUM_RES] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];
type Inv = [i32; NUM_RES];

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
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
fn cost(i: usize, inv: &Inv) -> f64 { (RESOURCES[i].cost_fn)(inv[i]) }
#[rustfmt::skip]
fn step(s: &GameState, t: i64) -> GameState { GameState { time: s.time + t, money: s.money + s.income * t as f64, ..*s } }

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

fn buy_next(s: &GameState, i: usize, goal: f64) -> Option<(GameState, i64)> {
    let wait = time_to_money(s, cost(i, &s.inventory));
    if wait == i64::MAX || s.time + wait >= finish_time(s, goal) {
        return None;
    }
    let mut ns = step(s, wait);
    ns.money -= cost(i, &ns.inventory);
    ns.inventory[i] += 1;
    ns.income = inc(&ns.inventory);
    Some((ns, finish_time(&ns, goal)))
}

fn reconstruct_path(mem: &HashMap<Inv, (i64, f64, usize)>, mut inv: Inv) -> Vec<String> {
    let mut log = Vec::new();
    while let Some(&(t, _, bought)) = mem.get(&inv) {
        if bought == usize::MAX {
            break;
        }
        log.push(format!("At time {}, bought {}, inventory {:?}", t, RESOURCES[bought].name, inv));
        inv[bought] -= 1; // Implicit step backward
    }
    log.reverse();
    log
}

#[derive(Clone, PartialEq, PartialOrd)]
struct Node(Reverse<i64>, GameState, usize);
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, o: &Self) -> Ordering {
        self.partial_cmp(o).unwrap()
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
    let mut mem: HashMap<Inv, (i64, f64, usize)> =
        HashMap::with_capacity_and_hasher(100_000, Default::default());

    let s0 = GameState::new();
    let mut q: BinaryHeap<_> = (0..NUM_RES).map(|i| Node(Reverse(0), s0, i)).collect();
    mem.insert(s0.inventory, (0, 0., usize::MAX));
    let mut best_time = finish_time(&s0, goal);
    let mut best_game = s0;
    let mut iter = 0;

    while let Some(Node(Reverse(_), s, i)) = q.pop() {
        iter += 1;
        if iter >= MAX_NODES {
            break;
        }
        if s.time >= best_time {
            continue;
        }
        let Some((ns, ns_finish)) = buy_next(&s, i, goal) else {
            continue;
        };
        if ns_finish >= finish_time(&s, goal) || ns.time >= best_time {
            continue;
        }

        let is_better = mem
            .get(&ns.inventory)
            .map_or(true, |&(mt, mm, _)| ns.time < mt || (ns.time == mt && ns.money > mm));
        if is_better {
            mem.insert(ns.inventory, (ns.time, ns.money, i));
            if ns_finish < best_time {
                best_time = ns_finish;
                best_game = ns;
                if verbose {
                    println!("Iter {iter}: New Best Time Found: {best_time}");
                }
            }

            for next_i in 0..NUM_RES {
                q.push(Node(Reverse(ns.time), ns, next_i));
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
