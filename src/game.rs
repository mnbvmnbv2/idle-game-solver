/// Domain model for idle-game rules.
///
/// This module intentionally has no solver/search/tracing logic.  It owns the
/// things that define "what game are we solving?": resources, prices,
/// production, initial state, and action application.

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
    pub goal: f64,
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
        assert!(self.goal.is_finite() && self.goal > 0.0, "goal must be positive and finite");
        assert!(
            self.initial_money.is_finite() && self.initial_money >= 0.0,
            "initial money must be non-negative and finite"
        );
        for (i, resource) in self.resources.iter().enumerate() {
            resource.validate(i);
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

#[derive(Clone, Debug)]
pub struct GameData {
    pub rules: GameRules,
    max_inventory: Inventory,
    cost: Vec<Vec<f64>>,
    yield_: Vec<Vec<f64>>,
    delta: Vec<Vec<f64>>,
}

impl GameData {
    pub fn new(rules: GameRules) -> Self {
        rules.validate();

        let mut max_inventory = rules.initial_inventory.clone();
        let (mut cost, mut yield_, mut delta) = (Vec::new(), Vec::new(), Vec::new());

        for (i, res) in rules.resources.iter().enumerate() {
            while (res.cost)(max_inventory[i]) < rules.goal {
                max_inventory[i] += 1;
                assert!(max_inventory[i] <= 1_000_000, "Resource {} has no cap", res.name);
            }

            let max_q = max_inventory[i];
            let ys: Vec<f64> = (0..=max_q + 1).map(|q| (res.production)(q)).collect();
            let cs: Vec<f64> = (0..=max_q).map(|q| (res.cost)(q)).collect();
            let ds: Vec<f64> = (0..=max_q).map(|q| ys[q as usize + 1] - ys[q as usize]).collect();

            cost.push(cs);
            yield_.push(ys);
            delta.push(ds);
        }

        Self { rules, max_inventory, cost, yield_, delta }
    }

    #[inline]
    pub fn resource_count(&self) -> usize {
        self.rules.resources.len()
    }

    #[inline]
    pub fn resource_name(&self, i: usize) -> &str {
        &self.rules.resources[i].name
    }

    #[inline]
    pub fn can_buy_more(&self, inv: &Inventory, i: usize) -> bool {
        inv[i] < self.max_inventory[i]
    }

    #[inline]
    pub fn cost(&self, i: usize, q: i32) -> f64 {
        self.cost[i][q as usize]
    }

    #[inline]
    pub fn delta(&self, i: usize, q: i32) -> f64 {
        self.delta[i][q as usize]
    }

    pub fn income(&self, inv: &Inventory) -> f64 {
        inv.iter().enumerate().map(|(i, &q)| self.yield_[i][q as usize]).sum()
    }

    pub fn initial_state(&self) -> GameState {
        let inventory = self.rules.initial_inventory.clone();
        GameState {
            time: 0,
            money: self.rules.initial_money,
            income: self.income(&inventory),
            inventory,
            progression: ProgressionState::base(),
        }
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

pub fn available_actions(data: &GameData, s: &GameState) -> Vec<Action> {
    let mut actions = Vec::new();
    for i in 0..data.resource_count() {
        if data.can_buy_more(&s.inventory, i) {
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
    data: &GameData,
    s: &GameState,
    i: usize,
    goal: f64,
    cost: f64,
    wait: i64,
) -> Option<ActionResult> {
    if wait == i64::MAX || s.time + wait >= finish_time(s, goal) {
        return None;
    }

    let mut ns = step(s, wait);
    ns.money -= cost;
    ns.inventory[i] += 1;
    ns.income += data.delta(i, s.inventory[i]);

    Some(ActionResult {
        action: Action::BuyResource(i),
        state: ns.clone(),
        finish_time: finish_time(&ns, goal),
        wait,
        cost_paid: cost,
    })
}

pub fn apply_action(
    data: &GameData,
    s: &GameState,
    action: Action,
    goal: f64,
    costs: &[f64],
    waits: &[i64],
) -> Option<ActionResult> {
    match action {
        Action::BuyResource(i) => buy_resource(data, s, i, goal, costs[i], waits[i]),
    }
}

pub fn next_buy_is_order_dominated(
    candidate: usize,
    costs: &[f64],
    waits: &[i64],
    deltas: &[f64],
) -> bool {
    let wait_c = waits[candidate];
    wait_c != i64::MAX
        && (0..costs.len()).any(|first| {
            first != candidate
                && waits[first] < wait_c
                && deltas[first] > 0.0
                && deltas[first] * (wait_c - waits[first]) as f64 + 1e-9 >= costs[first]
        })
}
