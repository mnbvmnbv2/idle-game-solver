mod tracing;

use rustc_hash::FxHashMap as HashMap;
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
    Resource {
        name: "Clicker",
        cost_fn: |q| 10. * 1.1_f64.powi(q),
        yield_fn: |q| 2. * q as f64,
    },
    Resource {
        name: "Factory",
        cost_fn: |q| 100. * 1.2_f64.powi(q),
        yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 },
    },
    Resource {
        name: "Depot",
        cost_fn: |q| 1000. * 1.3_f64.powi(q),
        yield_fn: |q| 210. * q as f64,
    },
];

pub type Inv = [i32; NUM_RES];

#[derive(Clone, Copy, Debug)]
pub struct GameState {
    pub time: i64,
    pub money: f64,
    pub inventory: Inv,
    pub income: f64,
}

impl GameState {
    fn new() -> Self {
        let inventory = [1, 0, 0];
        Self { time: 0, money: 0., inventory, income: inc(&inventory) }
    }
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
struct SearchBounds {
    max_inventory: Inv,
    max_income: f64,
    purchases: Vec<Vec<PurchaseInfo>>,
}
#[derive(Clone, Copy, Debug)]
struct PurchaseInfo {
    cost: f64,
    delta_income: f64,
}

impl SearchBounds {
    fn new(goal: f64) -> Self {
        let start = GameState::new();
        let mut max_inventory = start.inventory;

        for i in 0..NUM_RES {
            let mut q = max_inventory[i];

            while (RESOURCES[i].cost_fn)(q) < goal {
                q += 1;

                if q > 1_000_000 {
                    panic!(
                        "Resource {} appears to have no finite cap below goal {}",
                        RESOURCES[i].name, goal
                    );
                }
            }

            max_inventory[i] = q;
        }

        let mut max_income = 0.;

        for i in 0..NUM_RES {
            let mut best_yield = f64::NEG_INFINITY;

            for q in 0..=max_inventory[i] {
                best_yield = best_yield.max((RESOURCES[i].yield_fn)(q));
            }

            max_income += best_yield;
        }

        let mut purchases = Vec::with_capacity(NUM_RES);

        for i in 0..NUM_RES {
            let mut resource_purchases = Vec::new();

            for q in 0..max_inventory[i] {
                let y0 = (RESOURCES[i].yield_fn)(q);
                let y1 = (RESOURCES[i].yield_fn)(q + 1);

                resource_purchases
                    .push(PurchaseInfo { cost: (RESOURCES[i].cost_fn)(q), delta_income: y1 - y0 });
            }

            purchases.push(resource_purchases);
        }

        Self { max_inventory, max_income, purchases }
    }

    fn can_buy_more(&self, s: &GameState, i: usize) -> bool {
        s.inventory[i] < self.max_inventory[i]
    }

    fn deadline_upper_money(&self, s: &GameState, deadline: i64) -> f64 {
        if deadline <= s.time {
            return s.money;
        }

        let t = (deadline - s.time) as f64;

        let mut upper = s.money + s.income * t;

        for i in 0..NUM_RES {
            let start = s.inventory[i] as usize;
            let end = self.max_inventory[i] as usize;

            for p in &self.purchases[i][start..end] {
                if p.delta_income <= 0. {
                    continue;
                }

                let optimistic_net = p.delta_income * t - p.cost;

                if optimistic_net > 0. {
                    upper += optimistic_net;
                }
            }
        }

        upper
    }

    fn can_still_beat_best(&self, s: &GameState, goal: f64, best_time: i64) -> bool {
        if s.time >= best_time {
            return false;
        }

        // To improve the incumbent, we need to reach the goal strictly before best_time.
        let deadline = best_time - 1;

        // Small epsilon to avoid unsafe pruning from floating point roundoff.
        self.deadline_upper_money(s, deadline) + 1e-7 >= goal
    }
}

fn greedy_upper_bound(goal: f64, bounds: &SearchBounds) -> (i64, GameState) {
    let mut s = GameState::new();
    loop {
        let current_finish = finish_time(&s, goal);
        let mut best_buy: Option<BuyResult> = None;
        for i in 0..NUM_RES {
            if !bounds.can_buy_more(&s, i) {
                continue;
            }
            let Some(buy) = buy_next(&s, i, goal) else {
                continue;
            };
            if buy.finish_time >= current_finish {
                continue;
            }
            let is_best = best_buy.as_ref().map_or(true, |b| buy.finish_time < b.finish_time);
            if is_best {
                best_buy = Some(buy);
            }
        }
        let Some(buy) = best_buy else {
            return (current_finish, s);
        };
        s = buy.state;
    }
}

fn inc(inv: &Inv) -> f64 {
    inv.iter().enumerate().map(|(i, &q)| (RESOURCES[i].yield_fn)(q)).sum()
}
fn cost(i: usize, inv: &Inv) -> f64 {
    (RESOURCES[i].cost_fn)(inv[i])
}
fn step(s: &GameState, t: i64) -> GameState {
    GameState { time: s.time + t, money: s.money + s.income * t as f64, ..*s }
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

fn buy_next(s: &GameState, i: usize, goal: f64) -> Option<BuyResult> {
    let c = cost(i, &s.inventory);
    let wait = time_to_money(s, c);

    if wait == i64::MAX || s.time + wait >= finish_time(s, goal) {
        return None;
    }

    let mut ns = step(s, wait);
    ns.money -= cost(i, &ns.inventory);
    ns.inventory[i] += 1;
    ns.income = inc(&ns.inventory);

    Some(BuyResult { state: ns, finish_time: finish_time(&ns, goal), wait, cost_paid: c })
}

fn buy_first_dominates_second(s: &GameState, first: usize, second: usize) -> bool {
    if first == second {
        return false;
    }

    let first_cost = cost(first, &s.inventory);
    let second_cost = cost(second, &s.inventory);

    let wait_first = time_to_money(s, first_cost);
    let wait_second = time_to_money(s, second_cost);

    if wait_first == i64::MAX || wait_second == i64::MAX {
        return false;
    }

    // If first cannot be bought no later than second, it cannot dominate second-first.
    if wait_first > wait_second {
        return false;
    }

    let first_delta_income = (RESOURCES[first].yield_fn)(s.inventory[first] + 1)
        - (RESOURCES[first].yield_fn)(s.inventory[first]);

    if first_delta_income <= 0. {
        return false;
    }

    // State after buying second directly.
    let mut second_only = step(s, wait_second);
    second_only.money -= second_cost;
    second_only.inventory[second] += 1;
    second_only.income = inc(&second_only.inventory);

    // State after buying first.
    let mut first_then = step(s, wait_first);
    first_then.money -= first_cost;
    first_then.inventory[first] += 1;
    first_then.income = inc(&first_then.inventory);

    // Advance first-then branch to the exact time second-only bought second.
    let extra_wait = wait_second - wait_first;
    first_then = step(&first_then, extra_wait);

    // Since first != second and costs only depend on current quantity of that resource,
    // second's cost is unchanged by buying first.
    let second_cost_after_first = cost(second, &first_then.inventory);

    if first_then.money < second_cost_after_first {
        return false;
    }

    first_then.money -= second_cost_after_first;
    first_then.inventory[second] += 1;
    first_then.income = inc(&first_then.inventory);

    // Now both branches are at the same time and both have bought second.
    // The first-then branch also owns one extra `first`.
    //
    // If it has at least as much money, it dominates second-only.
    first_then.money + 1e-9 >= second_only.money
}

fn next_buy_is_order_dominated(s: &GameState, candidate: usize) -> Option<usize> {
    for first in 0..NUM_RES {
        if buy_first_dominates_second(s, first, candidate) {
            return Some(first);
        }
    }

    None
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
        // Reverse ordering because BinaryHeap is a max-heap.
        other.priority.cmp(&self.priority)
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn search<O: SearchObserver>(goal: f64, verbose: bool, observer: &mut O) -> SolveResult {
    let bounds = SearchBounds::new(goal);
    let (greedy_best_time, greedy_best_game) = greedy_upper_bound(goal, &bounds);

    let mut mem: HashMap<Inv, (i64, f64, usize, usize)> =
        HashMap::with_capacity_and_hasher(100_000, Default::default());

    let mut q = BinaryHeap::new();

    let s0 = GameState::new();
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

    mem.insert(s0.inventory, (0, 0., usize::MAX, root_id));

    q.push(Node { priority: s0.time, state: s0, id: root_id });

    let mut best_time = greedy_best_time;
    let mut best_game = greedy_best_game;
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

        // Lazy deletion:
        // If a faster path reached this inventory while this node waited in the queue, drop it.
        if let Some(&(mt, mm, _, best_mem_id)) = mem.get(&s.inventory) {
            if id != best_mem_id && (s.time > mt || (s.time == mt && s.money < mm)) {
                observer.prune(id, iter, "lazy_deleted_dominated_inventory");
                continue;
            }
        }

        if !bounds.can_still_beat_best(&s, goal, best_time) {
            observer.prune(id, iter, "deadline_npv_bound_cannot_beat_best");
            continue;
        }

        for i in 0..NUM_RES {
            if !bounds.can_buy_more(&s, i) {
                observer.reject_buy(id, iter, i, "resource_inventory_cap_reached");
                continue;
            }

            if let Some(_dominating_first) = next_buy_is_order_dominated(&s, i) {
                observer.reject_buy(id, iter, i, "next_buy_order_dominated");
                continue;
            }

            let Some(buy) = buy_next(&s, i, goal) else {
                observer.reject_buy(id, iter, i, "cannot_buy_before_current_finish");
                continue;
            };

            if buy.finish_time >= finish_time(&s, goal) {
                observer.reject_buy(id, iter, i, "does_not_improve_parent_finish");
                continue;
            }

            if buy.state.time >= best_time {
                observer.reject_buy(id, iter, i, "buy_time_after_best");
                continue;
            }

            let is_better = mem.get(&buy.state.inventory).map_or(true, |&(mt, mm, _, _)| {
                buy.state.time < mt || (buy.state.time == mt && buy.state.money > mm)
            });

            if !is_better {
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

            mem.insert(buy.state.inventory, (buy.state.time, buy.state.money, i, child_id));

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

        println!("Time elapsed: {:?}", start.elapsed());
        println!("{result:?}");

        match trace.write(path, goal, &result) {
            Ok(()) => println!("Wrote trace to {path}"),
            Err(e) => eprintln!("Failed to write trace: {e}"),
        }
    } else {
        let mut trace = NullTrace::default();
        let result = search(goal, VERBOSE, &mut trace);

        println!("Time elapsed: {:?}", start.elapsed());
        println!("{result:?}");
    }
}
