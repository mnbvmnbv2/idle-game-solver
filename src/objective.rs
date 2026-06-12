use crate::game::{time_to_money, GameBounds, GameData, GameRules, GameState, Inventory};

/// What the solver is trying to reach.
///
/// Keep this separate from `GameRules`: rules describe the game, while an
/// objective describes one benchmark/run target for that game.
#[derive(Clone, Debug)]
pub enum Objective {
    Money { amount: f64 },
    InventoryAtLeast { quantities: Inventory },
    IncomeAtLeast { amount: f64 },
}

impl Objective {
    pub fn money(amount: f64) -> Self {
        Self::Money { amount }
    }

    pub fn inventory_at_least(quantities: Inventory) -> Self {
        Self::InventoryAtLeast { quantities }
    }

    pub fn income_at_least(amount: f64) -> Self {
        Self::IncomeAtLeast { amount }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Money { amount } => format!("money:{amount}"),
            Self::InventoryAtLeast { quantities } => format!("inventory:{quantities:?}"),
            Self::IncomeAtLeast { amount } => format!("income:{amount}"),
        }
    }

    pub fn validate(&self, data: &GameData) {
        match self {
            Self::Money { amount } | Self::IncomeAtLeast { amount } => {
                assert!(
                    amount.is_finite() && *amount >= 0.0,
                    "objective amount must be finite and non-negative"
                );
            }
            Self::InventoryAtLeast { quantities } => {
                assert_eq!(
                    quantities.len(),
                    data.resource_count(),
                    "inventory objective length must match resource count"
                );
                assert!(
                    quantities.iter().all(|&q| q >= 0),
                    "inventory objective cannot contain negative quantities"
                );
            }
        }
    }

    pub fn is_satisfied(&self, state: &GameState) -> bool {
        match self {
            Self::Money { amount } => state.money >= *amount,
            Self::InventoryAtLeast { quantities } => {
                state.inventory.iter().zip(quantities.iter()).all(|(&have, &want)| have >= want)
            }
            Self::IncomeAtLeast { amount } => state.income >= *amount,
        }
    }

    /// Earliest known finish time from this state without taking another action.
    ///
    /// Money can be completed by waiting. Inventory/income cannot be completed
    /// by waiting, so their finish time is only finite once already satisfied.
    pub fn finish_time(&self, state: &GameState) -> i64 {
        match self {
            Self::Money { amount } => state.time.saturating_add(time_to_money(state, *amount)),
            Self::InventoryAtLeast { .. } | Self::IncomeAtLeast { .. } => {
                if self.is_satisfied(state) {
                    state.time
                } else {
                    i64::MAX
                }
            }
        }
    }

    /// Convert a winning state into the actual final state reported to callers.
    /// Money objectives may require waiting. Other objectives finish immediately.
    pub fn final_state(&self, state: &GameState) -> GameState {
        match self {
            Self::Money { amount } => crate::game::step(state, time_to_money(state, *amount)),
            Self::InventoryAtLeast { .. } | Self::IncomeAtLeast { .. } => state.clone(),
        }
    }

    /// Make sure precomputed resource tables are large enough for objective-specific targets.
    pub fn min_inventory_cap(&self, resource_count: usize) -> Inventory {
        match self {
            Self::InventoryAtLeast { quantities } => quantities.clone(),
            _ => vec![0; resource_count],
        }
    }

    /// Money-sized hint used for resource table caps.
    ///
    /// This preserves the old behavior for money goals while letting non-money
    /// objectives still use the scenario's default cap.
    pub fn money_cap_hint(&self) -> Option<f64> {
        match self {
            Self::Money { amount } => Some(*amount),
            _ => None,
        }
    }

    pub fn bounds_for_rules(&self, rules: &GameRules) -> GameBounds {
        let max_inventory = match self {
            Self::Money { amount } => rules
                .resources
                .iter()
                .enumerate()
                .map(|(i, resource)| {
                    let mut q = rules.initial_inventory[i];

                    while (resource.cost)(q) < *amount {
                        q += 1;
                        assert!(q <= 1_000_000, "Resource {} has no cap", resource.name);
                    }

                    q
                })
                .collect(),

            Self::InventoryAtLeast { quantities } => {
                assert_eq!(
                    quantities.len(),
                    rules.resources.len(),
                    "inventory objective length must match resource count"
                );

                quantities
                    .iter()
                    .zip(rules.initial_inventory.iter())
                    .map(|(&want, &initial)| want.max(initial))
                    .collect()
            }

            Self::IncomeAtLeast { amount } => rules
                .resources
                .iter()
                .enumerate()
                .map(|(i, resource)| {
                    let mut q = rules.initial_inventory[i];

                    // Conservative finite cap: enough of this one resource alone
                    // could satisfy the income target.
                    while (resource.production)(q) < *amount {
                        q += 1;
                        assert!(q <= 1_000_000, "Resource {} has no cap", resource.name);
                    }

                    q
                })
                .collect(),
        };

        GameBounds { max_inventory }
    }
}
