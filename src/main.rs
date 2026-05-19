use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::iter::zip;
use std::time::Instant;

struct Resource {
    name: &'static str,
    cost_fn: fn(i32) -> f64,  // Function: f(x) -> cost of the next unit
    yield_fn: fn(i32) -> f64, // Function: g(x) -> total production per second
}

const NUM_RES: usize = 3;
const RESOURCES: [Resource; 3] = [
    Resource {
        name: "Clicker",
        cost_fn: |q| 10.0 * 1.1_f64.powi(q),
        yield_fn: |q| 2.0 * (q as f64),
    },
    Resource {
        name: "Factory",
        cost_fn: |q| 100.0 * 1.2_f64.powi(q),
        yield_fn: |q| {
            if q >= 5 {
                30.0 * (q as f64)
            } else {
                10.0 * (q as f64)
            }
        },
    },
    Resource {
        name: "Depot",
        cost_fn: |q| 1000.0 * 1.3_f64.powi(q),
        yield_fn: |q| 210.0 * (q as f64),
    },
];

type Inventory = [i32; NUM_RES];

#[derive(Clone, Copy, Debug)]
struct GameState {
    time: i64,
    money: f64,
    inventory: Inventory,
}

impl GameState {
    fn new() -> GameState {
        let mut gs = GameState {
            time: 0,
            money: 0.0,
            inventory: [0; NUM_RES],
        };
        gs.inventory[0] = 1;
        gs
    }
}

// --- transitions and helpers ---

fn get_income(state: &GameState) -> f64 {
    let mut income = 0.;
    for (r, q) in zip(&RESOURCES, state.inventory) {
        income += (r.yield_fn)(q)
    }
    income
}
fn get_cost(idx: usize, s: &GameState) -> f64 {
    (RESOURCES[idx].cost_fn)(s.inventory[idx])
}
fn step(s: GameState, ticks: i64) -> GameState {
    GameState {
        time: s.time + ticks,
        money: s.money + (get_income(&s) * ticks as f64),
        inventory: s.inventory,
    }
}

fn buy(state: &GameState, idx: usize) -> Option<GameState> {
    let price = get_cost(idx, state);

    if state.money + 1e-9 < price {
        return None;
    }
    let mut new_state = *state;

    new_state.money -= price;
    new_state.inventory[idx] += 1;

    Some(new_state)
}

// --- solver stuff ---

fn time_to_money(s: &GameState, money: f64) -> i64 {
    let income = get_income(&s);
    if income <= 0.0 {
        return i64::MAX;
    }

    ((money - s.money).max(0.0) / income).ceil() as i64
}

struct BuyOrder {
    game: GameState,
    time: i64,
    done: bool,
}

fn buy_order(s: GameState, order: usize, goal: f64) -> BuyOrder {
    let time_to_goal = time_to_money(&s, goal);
    let time_to_resource = time_to_money(&s, get_cost(order, &s));
    if time_to_goal <= time_to_resource {
        return BuyOrder {
            game: s,
            time: s.time + time_to_goal,
            done: true,
        };
    }

    let s = buy(&step(s, time_to_resource), order).unwrap();
    BuyOrder {
        game: s,
        time: s.time + time_to_money(&s, goal),
        done: false,
    }
}

// main

fn reconstruct_path(
    memory: &HashMap<Inventory, MemEntry>,
    final_inventory: Inventory,
) -> Vec<String> {
    let mut log = Vec::new();
    let mut curr_inv = final_inventory;

    while let Some(&(time, _money, action)) = memory.get(&curr_inv) {
        if action == usize::MAX {
            break;
        }

        log.push(format!(
            "Reached {:?} by buying {} at time {}",
            curr_inv, RESOURCES[action].name, time
        ));

        curr_inv[action] -= 1;
    }

    log.reverse();
    log
}

struct QueueNode {
    priority: i64,
    game: GameState,
    order: usize,
}

impl PartialEq for QueueNode {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}
impl Eq for QueueNode {}

impl Ord for QueueNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}
impl PartialOrd for QueueNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type MemEntry = (i64, f64, usize);

fn dijkstra(goal: f64) {
    let start_game = GameState::new();

    let mut memory: HashMap<Inventory, MemEntry> = HashMap::new();
    memory.insert(
        start_game.inventory,
        (start_game.time, start_game.money, usize::MAX),
    );

    let mut best_finish_time = start_game.time + time_to_money(&start_game, goal);
    let mut best_game = start_game;

    let mut pq: BinaryHeap<QueueNode> = BinaryHeap::new();

    for idx in 0..NUM_RES {
        pq.push(QueueNode {
            priority: start_game.time,
            game: start_game,
            order: idx,
        });
    }

    let mut iter = 0;

    while let Some(node) = pq.pop() {
        if iter >= 10_000_000 {
            break;
        }
        iter += 1;

        if node.priority >= best_finish_time {
            continue;
        }

        let curr_game = node.game;
        let order = node.order;

        let bo = buy_order(curr_game, order, goal);
        let next_game = bo.game;
        let finish_time = bo.time;
        let done = bo.done;

        let is_worse_than_parent = finish_time >= curr_game.time + time_to_money(&curr_game, goal);
        if is_worse_than_parent {
            continue;
        }

        let mut is_better_than_memory = true;

        if let Some(&(best_mem_time, best_mem_money, _)) = memory.get(&next_game.inventory) {
            is_better_than_memory = (next_game.time < best_mem_time)
                || (next_game.time == best_mem_time && next_game.money > best_mem_money);
        }

        if is_better_than_memory {
            memory.insert(
                next_game.inventory,
                (next_game.time, next_game.money, order),
            );

            if !done && next_game.time < best_finish_time {
                for idx in 0..NUM_RES {
                    pq.push(QueueNode {
                        priority: next_game.time,
                        game: next_game,
                        order: idx,
                    });
                }
            }
        }

        if finish_time < best_finish_time {
            best_finish_time = finish_time;
            best_game = next_game;
            println!("Iter: {}: New Best Time Found: {}", iter, best_finish_time);
        }
    }

    best_game = step(best_game, time_to_money(&best_game, goal));

    let path = reconstruct_path(&memory, best_game.inventory);
    println!("\nBest history:\n{}", path.join("\n"));

    println!(
        "Final Wealth: {:.2} in {}",
        best_game.money, best_finish_time
    );
    println!("Did {} iterations", iter);
}

fn main() {
    let start = Instant::now();
    dijkstra(1e11_f64);
    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);
}
