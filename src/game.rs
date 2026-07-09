/// Domain model for idle-game rules.
use crate::objective::Objective;

pub type Inventory = Vec<i32>;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ProgressionState {
    pub upgrade_mask: u64,
    pub ascensions: i32,
}

impl ProgressionState {
    pub fn base() -> Self {
        Self { upgrade_mask: 0, ascensions: 0 }
    }
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub time: i64,
    pub money: f64,
    pub inventory: Inventory,
    pub progression: ProgressionState,
    pub income: f64,
}

#[derive(Clone, Debug)]
pub struct GameRules {
    pub name: String,
    pub initial_money: f64,
    pub initial_inventory: Inventory,
    pub resources: Vec<ResourceRule>,
}

impl GameRules {
    pub fn validate(&self) {
        assert!(!self.resources.is_empty(), "game must define at least one resource");
        assert_eq!(
            self.initial_inventory.len(),
            self.resources.len(),
            "initial inventory length must match resource count"
        );
        assert!(
            self.initial_money.is_finite() && self.initial_money >= 0.0,
            "initial money must be non-negative and finite"
        );
        for (i, resource) in self.resources.iter().enumerate() {
            resource.validate(i);
        }
    }

    #[inline]
    pub fn resource_count(&self) -> usize {
        self.resources.len()
    }

    #[inline]
    pub fn resource_name(&self, i: usize) -> &str {
        &self.resources[i].name
    }

    #[inline]
    pub fn cost(&self, i: usize, q: i32) -> f64 {
        (self.resources[i].cost)(q)
    }

    #[inline]
    pub fn delta(&self, i: usize, quantity: i32) -> f64 {
        let production = self.resources[i].production;
        production(quantity + 1) - production(quantity)
    }

    pub fn income(&self, inv: &Inventory) -> f64 {
        inv.iter().enumerate().map(|(i, &q)| (self.resources[i].production)(q)).sum()
    }

    pub fn initial_state(&self) -> GameState {
        let inventory = self.initial_inventory.clone();
        GameState {
            time: 0,
            money: self.initial_money,
            income: self.income(&inventory),
            inventory,
            progression: ProgressionState::base(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResourceRule {
    pub name: String,
    pub cost: fn(i32) -> f64,
    pub production: fn(i32) -> f64,
}

impl ResourceRule {
    fn validate(&self, i: usize) {
        assert!(!self.name.is_empty(), "resource {i} must have a name");
        assert!((self.cost)(0).is_finite(), "resource {} has invalid initial cost", self.name);
        assert!(
            (self.production)(1).is_finite(),
            "resource {} has invalid initial production",
            self.name
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    BuyResource(usize),
    // Later:
    // BuyUpgrade(usize),
    // Ascend,
}

#[derive(Clone, Debug)]
pub struct ActionResult {
    pub action: Action,
    pub state: GameState,
    pub finish_time: i64,
    pub wait: i64,
    pub cost_paid: f64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct MemoKey {
    pub inventory: Inventory,
    pub progression: ProgressionState,
}

impl MemoKey {
    #[inline]
    pub fn from_state(s: &GameState) -> Self {
        Self { inventory: s.inventory.clone(), progression: s.progression.clone() }
    }

    #[inline]
    pub fn from_inventory(inventory: Inventory) -> Self {
        Self { inventory, progression: ProgressionState::base() }
    }
}

#[inline]
pub fn step(s: &GameState, t: i64) -> GameState {
    let mut next = s.clone();
    next.time += t;
    next.money += s.income * t as f64;
    next
}

#[inline]
pub fn time_to_money(s: &GameState, goal: f64) -> i64 {
    if s.money >= goal {
        0
    } else if s.income <= 0.0 {
        i64::MAX
    } else {
        ((goal - s.money) / s.income).ceil() as i64
    }
}

#[inline]
pub fn finish_time(s: &GameState, goal: f64) -> i64 {
    s.time.saturating_add(time_to_money(s, goal))
}

pub fn available_actions(
    rules: &GameRules,
    max_inventory: &Inventory,
    s: &GameState,
) -> Vec<Action> {
    let mut actions = Vec::new();
    for i in 0..rules.resource_count() {
        if s.inventory[i] < max_inventory[i] {
            actions.push(Action::BuyResource(i));
        }
    }
    actions
}

#[inline]
pub fn action_resource_index(action: Action) -> Option<usize> {
    match action {
        Action::BuyResource(i) => Some(i),
    }
}

fn buy_resource(
    rules: &GameRules,
    s: &GameState,
    i: usize,
    objective: &Objective,
    current_finish: i64,
    cost: f64,
    wait: i64,
) -> Option<ActionResult> {
    if wait == i64::MAX {
        return None;
    }

    if current_finish != i64::MAX && s.time.saturating_add(wait) >= current_finish {
        return None;
    }

    let mut ns = step(s, wait);
    ns.money -= cost;
    ns.inventory[i] += 1;
    ns.income += rules.delta(i, s.inventory[i]);

    Some(ActionResult {
        action: Action::BuyResource(i),
        state: ns.clone(),
        finish_time: objective.finish_time(&ns),
        wait,
        cost_paid: cost,
    })
}

pub fn apply_action(
    rules: &GameRules,
    s: &GameState,
    action: Action,
    objective: &Objective,
    current_finish: i64,
    costs: &[f64],
    waits: &[i64],
) -> Option<ActionResult> {
    match action {
        Action::BuyResource(i) => {
            buy_resource(rules, s, i, objective, current_finish, costs[i], waits[i])
        }
    }
}
