//! Solver for idle-games with assumptions:
//! - We assume no income decreases
//!

use std::ops::Bound::{Excluded, Unbounded};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap},
    time::Instant,
};

const GOAL: f64 = 1e12;
const NUM_RES: usize = 3;
const MAX_NODES: usize = 10_000_000;
const VERBOSE: bool = false;

#[rustfmt::skip]
struct Resource { name: &'static str, cost_fn: fn(i32) -> f64, yield_fn: fn(i32) -> f64 }

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
    income: f64,
}

impl GameState {
    fn new() -> Self {
        let inventory = [1, 0, 0];
        let income = get_inc(&inventory);
        Self { time: 0, money: 0., inventory, income }
    }
}

#[rustfmt::skip]
fn get_inc(inv: &[i32; NUM_RES]) -> f64 { inv.iter().enumerate().map(|(i, &q)| (RESOURCES[i].yield_fn)(q)).sum() }

#[rustfmt::skip]
fn get_cost(idx: usize, s: &GameState) -> f64 { (RESOURCES[idx].cost_fn)(s.inventory[idx]) }

#[rustfmt::skip]
fn step(s: GameState, t: i64) -> GameState { GameState { time: s.time + t, money: s.money + s.income * t as f64, ..s } }

fn buy(mut state: GameState, idx: usize) -> Option<GameState> {
    let price = get_cost(idx, &state);
    (state.money >= price).then(|| {
        state.money -= price;
        state.inventory[idx] += 1;
        state.income = get_inc(&state.inventory);
        state
    })
}

fn time_to_money(s: &GameState, goal: f64) -> i64 {
    let inc = s.income;
    if inc <= 0. {
        i64::MAX
    } else {
        ((goal - s.money).max(0.) / inc).ceil() as i64
    }
}

fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time + time_to_money(s, goal)
}

/// Returns (state after buying, finish time after buying), unless buying is not useful.
fn try_buy_next(s: GameState, order: usize, goal: f64) -> Option<(GameState, i64)> {
    let current_finish_time = finish_time(&s, goal);
    let time_to_resource = time_to_money(&s, get_cost(order, &s));
    let resource_time = s.time + time_to_resource;

    if current_finish_time <= resource_time {
        return None;
    }

    let s = buy(step(s, time_to_resource), order).unwrap();
    Some((s, finish_time(&s, goal)))
}

type StateKey = (i64, [i32; NUM_RES]);

fn reconstruct_path(
    mem: &BTreeMap<StateKey, (GameState, usize, StateKey)>,
    mut key: StateKey,
) -> Vec<String> {
    let mut log = Vec::new();
    while let Some(&(s, o, parent_key)) = mem.get(&key) {
        if s.time == 0 {
            break;
        }
        log.push(format!(
            "Reached {:?} by buying {} at time {}",
            s.inventory, RESOURCES[o].name, key.0
        ));
        key = parent_key;
    }
    log.reverse();
    log
}

// Node implementation (time, state)
struct Node(i64, GameState);

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

fn try_queue_next(
    pri_q: &mut BinaryHeap<Node>,
    mem: &mut BTreeMap<StateKey, (GameState, usize, StateKey)>,
    best_time: &mut i64,
    best_game: &mut GameState,
    current_state: GameState,
    order: usize,
    goal: f64,
    verbose: bool,
    iter: usize,
) {
    let current_finish_time = finish_time(&current_state, goal);

    let Some((next_state, next_complete_time)) = try_buy_next(current_state, order, goal) else {
        return;
    };

    // if we are already later than best time
    if current_state.time >= *best_time {
        return;
    }

    // if we end up after best time
    if next_state.time >= *best_time {
        return;
    }

    // if the purchase made us worse off
    if next_complete_time >= current_finish_time {
        return;
    }

    let key = (next_state.time, next_state.inventory);
    let parent_key = (current_state.time, current_state.inventory);

    if let Some((mem_state, _, _)) = mem.get(&key) {
        if mem_state.income > next_state.income
            || (mem_state.income == next_state.income && mem_state.money > next_state.money)
        {
            return;
        }
    }

    if let Some((_, (mem_state, _, _))) = mem.range((Unbounded, Excluded(&key))).next_back() {
        if mem_state.income > next_state.income
            || (mem_state.income == next_state.income && mem_state.money > next_state.money)
        {
            return;
        }
    }

    mem.insert(key, (next_state, order, parent_key));
    pri_q.push(Node(next_state.time, next_state));

    // if better we update
    if next_complete_time < *best_time {
        *best_time = next_complete_time;
        *best_game = next_state;

        if verbose {
            println!("Iter: {iter}: New Best Time Found: {best_time}");
        }
    }
}

fn search(goal: f64, verbose: bool) -> SolveResult {
    let mut mem: BTreeMap<StateKey, (GameState, usize, StateKey)> = BTreeMap::new();
    let s0 = GameState::new();
    let k0 = (s0.time, s0.inventory);
    mem.insert(k0, (s0, usize::MAX, k0));

    let (mut best_time, mut best_game) = (finish_time(&s0, goal), s0);
    let mut pri_q: BinaryHeap<_> = BinaryHeap::new();
    let mut iter = 0;

    for i in 0..NUM_RES {
        try_queue_next(
            &mut pri_q,
            &mut mem,
            &mut best_time,
            &mut best_game,
            s0,
            i,
            goal,
            verbose,
            iter,
        );
    }

    while let Some(Node(_, current_state)) = pri_q.pop() {
        iter += 1;
        if iter >= MAX_NODES {
            break;
        }

        // add next decisions
        for i in 0..NUM_RES {
            try_queue_next(
                &mut pri_q,
                &mut mem,
                &mut best_time,
                &mut best_game,
                current_state,
                i,
                goal,
                verbose,
                iter,
            );
        }
    }

    if verbose {
        let final_key = (best_game.time, best_game.inventory);
        for line in reconstruct_path(&mem, final_key) {
            println!("{line}");
        }
    }

    best_game = step(best_game, time_to_money(&best_game, goal));

    SolveResult {
        best_time,
        final_money: best_game.money,
        iterations: iter,
        final_inventory: best_game.inventory,
    }
}

fn main() {
    let start = Instant::now();
    let result = search(GOAL, VERBOSE);
    println!("Time elapsed: {:?}", start.elapsed());
    println!("{result:?}");
}
