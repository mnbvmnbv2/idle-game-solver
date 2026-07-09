use rustc_hash::FxHashMap;
use std::{cmp::Ordering, collections::BinaryHeap};

use crate::{
    game::{
        action_resource_index, apply_action, available_actions, next_buy_is_order_dominated,
        time_to_money, Action, GameRules, GameState, Inventory, MemoKey,
    },
    objective::Objective,
    tracing::{AcceptedNode, SearchObserver},
};

#[derive(Debug, Clone)]
pub struct SolveResult {
    pub best_time: i64,
    pub final_money: f64,
    pub iterations: usize,
    pub final_inventory: Inventory,
}

#[derive(Clone, Debug)]
pub struct SearchConfig {
    pub max_nodes: usize,
    pub verbose: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self { max_nodes: 10_000_000, verbose: false }
    }
}

/// Solver interface so future algorithms can be benchmarked against the same
/// `GameRules` cases and trace observer shape.
pub trait SolverAlgorithm {
    fn name(&self) -> &'static str;
    fn solve<O: SearchObserver>(
        &self,
        rules: GameRules,
        objective: Objective,
        observer: &mut O,
    ) -> SolveResult;
}

#[derive(Clone, Debug, Default)]
pub struct BranchAndBoundSolver {
    pub config: SearchConfig,
}

impl BranchAndBoundSolver {
    pub fn new(config: SearchConfig) -> Self {
        Self { config }
    }

    fn reconstruct_path(
        &self,
        rules: &GameRules,
        mem: &MemoTable,
        mut inv: Inventory,
    ) -> Vec<String> {
        let mut log = Vec::new();
        while let Some(m) = mem.get_inventory(inv.clone()) {
            if m.bought == usize::MAX {
                break;
            }
            log.push(format!(
                "At time {}, bought {}, inventory {:?}",
                m.time,
                rules.resource_name(m.bought),
                inv
            ));
            inv[m.bought] -= 1;
        }
        log.reverse();
        log
    }
}

impl SolverAlgorithm for BranchAndBoundSolver {
    fn name(&self) -> &'static str {
        "branch-and-bound"
    }

    fn solve<O: SearchObserver>(
        &self,
        rules: GameRules,
        objective: Objective,
        observer: &mut O,
    ) -> SolveResult {
        rules.validate();
        let max_inventory = objective.max_inventory_for_rules(&rules);

        objective.validate();
        let mut mem = MemoTable::new();
        let mut q = BinaryHeap::new();
        let s0 = rules.initial_state();
        let s0_finish = objective.finish_time(&s0);

        let root_id = observer.accept_node(AcceptedNode {
            parent: None,
            bought: None,
            iter_created: 0,
            state: s0.clone(),
            finish_time: s0_finish,
            wait: None,
            cost_paid: None,
        });

        observer.start(root_id, s0_finish);
        mem.insert_state(
            &s0,
            MemoEntry { time: 0, money: s0.money, bought: usize::MAX, node_id: root_id },
        );
        q.push(Node { priority: s0.time, state: s0.clone(), id: root_id });

        let mut best_time = s0_finish;
        let mut best_game = s0;
        let mut best_node_id = root_id;
        let mut iter = 0;

        while let Some(Node { state: s, id, .. }) = q.pop() {
            iter += 1;

            if iter >= self.config.max_nodes {
                observer.truncated(iter, "max_nodes_reached");
                break;
            }

            observer.pop(id, iter, q.len(), best_time);

            if s.time >= best_time {
                observer.prune(id, iter, "state_time_not_better_than_best");
                continue;
            }

            if let Some(m) = mem.get_state(&s) {
                if id != m.node_id && (s.time > m.time || (s.time == m.time && s.money < m.money)) {
                    observer.prune(id, iter, "lazy_deleted_dominated_state");
                    continue;
                }
            }

            let current_finish = objective.finish_time(&s);
            let res_count = rules.resource_count();
            let mut costs = vec![0.0; res_count];
            let mut waits = vec![i64::MAX; res_count];
            let mut deltas = vec![0.0; res_count];

            for i in 0..res_count {
                if objective.can_buy_resource(&s, i) {
                    costs[i] = rules.cost(i, s.inventory[i]);
                    waits[i] = time_to_money(&s, costs[i]);
                    deltas[i] = rules.delta(i, s.inventory[i]);
                }
            }

            for action in available_actions(&rules, &max_inventory, &s) {
                let Some(resource_i) = action_resource_index(action) else {
                    continue;
                };
                if next_buy_is_order_dominated(resource_i, &costs, &waits, &deltas) {
                    observer.reject_buy(id, iter, resource_i, "next_buy_order_dominated");
                    continue;
                }

                let Some(result) =
                    apply_action(&rules, &s, action, &objective, current_finish, &costs, &waits)
                else {
                    observer.reject_buy(id, iter, resource_i, "cannot_buy_before_current_finish");
                    continue;
                };

                if current_finish != i64::MAX && result.finish_time >= current_finish {
                    observer.reject_buy(id, iter, resource_i, "does_not_improve_parent_finish");
                    continue;
                }

                if result.state.time >= best_time {
                    observer.reject_buy(id, iter, resource_i, "buy_time_after_best");
                    continue;
                }

                if !mem.is_better_state(&result.state) {
                    observer.reject_buy(id, iter, resource_i, "dominated_state");
                    continue;
                }

                let bought_resource = match result.action {
                    Action::BuyResource(i) => i,
                };

                let child_id = observer.accept_node(AcceptedNode {
                    parent: Some(id),
                    bought: Some(bought_resource),
                    iter_created: iter,
                    state: result.state.clone(),
                    finish_time: result.finish_time,
                    wait: Some(result.wait),
                    cost_paid: Some(result.cost_paid),
                });

                observer.accept_buy(child_id, id, iter, bought_resource, result.finish_time);
                mem.insert_state(
                    &result.state,
                    MemoEntry {
                        time: result.state.time,
                        money: result.state.money,
                        bought: bought_resource,
                        node_id: child_id,
                    },
                );

                if result.finish_time < best_time {
                    best_time = result.finish_time;
                    best_game = result.state.clone();
                    best_node_id = child_id;

                    observer.best(child_id, iter, best_time);
                    if self.config.verbose {
                        println!("Iter {iter}: New Best Time Found: {best_time}");
                    }
                }

                q.push(Node { priority: result.state.time, state: result.state, id: child_id });
            }
        }

        if self.config.verbose {
            for line in self.reconstruct_path(&rules, &mem, best_game.inventory.clone()) {
                println!("{line}");
            }
            println!("Objective {} reached at time {best_time}.", objective.label());
        }

        let final_state = objective.final_state(&best_game);
        observer.finish(best_node_id);

        SolveResult {
            best_time,
            final_money: final_state.money,
            iterations: iter,
            final_inventory: final_state.inventory,
        }
    }
}

#[derive(Clone, Debug)]
struct MemoEntry {
    time: i64,
    money: f64,
    bought: usize,
    node_id: usize,
}

#[derive(Debug)]
struct MemoTable {
    data: FxHashMap<MemoKey, MemoEntry>,
}

impl MemoTable {
    fn new() -> Self {
        Self { data: FxHashMap::default() }
    }

    #[inline]
    fn get_key(&self, key: &MemoKey) -> Option<MemoEntry> {
        self.data.get(key).cloned()
    }

    #[inline]
    fn get_state(&self, state: &GameState) -> Option<MemoEntry> {
        self.get_key(&MemoKey::from_state(state))
    }

    #[inline]
    fn get_inventory(&self, inventory: Inventory) -> Option<MemoEntry> {
        self.get_key(&MemoKey::from_inventory(inventory))
    }

    #[inline]
    fn insert_key(&mut self, key: MemoKey, entry: MemoEntry) {
        self.data.insert(key, entry);
    }

    #[inline]
    fn insert_state(&mut self, state: &GameState, entry: MemoEntry) {
        self.insert_key(MemoKey::from_state(state), entry);
    }

    #[inline]
    fn is_better_key(&self, key: &MemoKey, time: i64, money: f64) -> bool {
        match self.data.get(key) {
            Some(m) => time < m.time || (time == m.time && money > m.money),
            None => true,
        }
    }

    #[inline]
    fn is_better_state(&self, state: &GameState) -> bool {
        self.is_better_key(&MemoKey::from_state(state), state.time, state.money)
    }
}

#[derive(Clone, Debug)]
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
