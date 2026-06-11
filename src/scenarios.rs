use crate::game::{GameRules, ResourceRule};

pub fn default_rules() -> GameRules {
    rules_with_goal(1e12)
}

pub fn rules_with_goal(goal: f64) -> GameRules {
    GameRules {
        name: "default".to_string(),
        goal,
        initial_money: 0.0,
        initial_inventory: vec![1, 0, 0],
        resources: vec![
            ResourceRule {
                name: "Clicker".to_string(),
                cost: |q| 10. * 1.1_f64.powi(q),
                production: |q| 2. * q as f64,
            },
            ResourceRule {
                name: "Factory".to_string(),
                cost: |q| 100. * 1.2_f64.powi(q),
                production: |q| if q >= 5 { 30. * q as f64 } else { 10. * q as f64 },
            },
            ResourceRule {
                name: "Depot".to_string(),
                cost: |q| 1000. * 1.3_f64.powi(q),
                production: |q| 210. * q as f64,
            },
        ],
    }
}
pub fn tiny_rules(goal: f64) -> GameRules {
    GameRules {
        name: "tiny".to_string(),
        goal,
        initial_money: 0.0,
        initial_inventory: vec![1, 0],
        resources: vec![
            ResourceRule {
                name: "Cursor".to_string(),
                cost: |q| 5. * 1.15_f64.powi(q),
                production: |q| 1. * q as f64,
            },
            ResourceRule {
                name: "Worker".to_string(),
                cost: |q| 30. * 1.25_f64.powi(q),
                production: |q| 8. * q as f64,
            },
        ],
    }
}

pub fn high_factory_rules(goal: f64) -> GameRules {
    let mut rules = rules_with_goal(goal);
    rules.name = "high_factory".to_string();
    rules.resources[1].production = |q| if q >= 4 { 45. * q as f64 } else { 12. * q as f64 };
    rules
}

pub fn named(name: &str, goal_override: Option<f64>) -> Option<GameRules> {
    let goal = goal_override.unwrap_or(1e12);
    match name {
        "default" => Some(rules_with_goal(goal)),
        "tiny" => Some(tiny_rules(goal_override.unwrap_or(10_000.0))),
        "high_factory" => Some(high_factory_rules(goal)),
        _ => None,
    }
}

pub fn names() -> &'static [&'static str] {
    &["default", "tiny", "high_factory"]
}
