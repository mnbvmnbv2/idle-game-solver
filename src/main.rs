use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    time::Instant,
};

#[rustfmt::skip]
struct Resource { name: &'static str, cost_fn: fn(i32) -> f64, yield_fn: fn(i32) -> f64 }
const NUM_RES: usize = 3;

#[rustfmt::skip]
const RESOURCES: [Resource; 3] = [
    Resource { name: "Clicker", cost_fn: |q| 10. * 1.1_f64.powi(q), yield_fn: |q| 2. * q as f64 },
    Resource { name: "Factory", cost_fn: |q| 100. * 1.2_f64.powi(q), yield_fn: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 } },
    Resource { name: "Depot", cost_fn: |q| 1000. * 1.3_f64.powi(q), yield_fn: |q| 210. * q as f64 },
];

#[inline(always)]
fn name(i: usize) -> String {
    match i {
        0 => "Clicker".to_string(),
        1 => "Factory".to_string(),
        2 => "Depot".to_string(),
        _ => unreachable!(),
    }
}

#[inline(always)]
fn cost(i: usize, q: i32) -> f64 {
    match i {
        0 => 10.0 * 1.1_f64.powi(q),
        1 => 100.0 * 1.2_f64.powi(q),
        2 => 1000.0 * 1.3_f64.powi(q),
        _ => unreachable!(),
    }
}

#[inline(always)]
fn yield_of(i: usize, q: i32) -> f64 {
    match i {
        0 => 2.0 * q as f64,
        1 => {
            if q >= 5 {
                30.0 * q as f64
            } else {
                10.0 * q as f64
            }
        }
        2 => 210.0 * q as f64,
        _ => unreachable!(),
    }
}

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
fn get_inc(s: &GameState) -> f64 { s.inventory.iter().enumerate().map(|(i, &q)| yield_of(i,q)).sum() }
#[rustfmt::skip]
fn get_cost(i: usize, s: &GameState) -> f64 { cost(i, s.inventory[i]) }
#[rustfmt::skip]
fn step(s: GameState, t: i64) -> GameState { GameState { time: s.time + t, money: s.money + get_inc(&s) * t as f64, ..s } }

fn buy(mut s: GameState, i: usize) -> Option<GameState> {
    let p = get_cost(i, &s);
    (s.money + 1e-9 >= p).then(|| {
        s.money -= p;
        s.inventory[i] += 1;
        s
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

fn buy_order(s: GameState, order: usize, goal: f64) -> (GameState, i64, bool) {
    let (tg, tr) = (time_to_money(&s, goal), time_to_money(&s, get_cost(order, &s)));
    if tg <= tr {
        return (s, s.time + tg, true);
    }
    let s = buy(step(s, tr), order).unwrap();
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
        log.push(format!("Reached {:?} by buying {} at time {}", inv, name(a), t));
        inv[a] -= 1;
    }
    log.reverse();
    log
}

// Minimal wrapper to force BinaryHeap to be a min-heap by priority/time only.
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

fn dijkstra(goal: f64) {
    let (mut mem, s0) = (HashMap::new(), GameState::new());
    mem.insert(s0.inventory, (s0.time, s0.money, usize::MAX));

    let (mut best_t, mut best_g) = (s0.time + time_to_money(&s0, goal), s0);
    let mut pq: BinaryHeap<_> = (0..NUM_RES).map(|i| Node(s0.time, s0, i)).collect();
    let mut iter = 0;

    while let Some(Node(pri, cg, order)) = pq.pop() {
        iter += 1;
        if iter >= 10_000_000 || pri >= best_t {
            if iter >= 10_000_000 {
                break;
            }
            continue;
        }

        let (ng, ft, done) = buy_order(cg, order, goal);
        if ft >= cg.time + time_to_money(&cg, goal) {
            continue;
        }

        if mem
            .get(&ng.inventory)
            .map_or(true, |&(mt, mm, _)| ng.time < mt || (ng.time == mt && ng.money > mm))
        {
            mem.insert(ng.inventory, (ng.time, ng.money, order));
            if !done && ng.time < best_t {
                pq.extend((0..NUM_RES).map(|i| Node(ng.time, ng, i)));
            }
        }
        if ft < best_t {
            best_t = ft;
            best_g = ng;
            println!("Iter: {iter}: New Best Time Found: {best_t}");
        }
    }

    best_g = step(best_g, time_to_money(&best_g, goal));
    println!("\nBest history:\n{}", reconstruct_path(&mem, best_g.inventory).join("\n"));
    println!("Final Wealth: {:.2} in {}\nDid {} iterations", best_g.money, best_t, iter);
}

fn main() {
    let start = Instant::now();
    dijkstra(1e11);
    println!("Time elapsed: {:?}", start.elapsed());
}
