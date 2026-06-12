use crate::game::{GameRules, ResourceRule};

pub fn default_rules() -> GameRules {
    GameRules {
        name: "default".to_string(),
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
pub fn tiny_rules() -> GameRules {
    GameRules {
        name: "tiny".to_string(),
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

pub fn high_factory_rules() -> GameRules {
    let mut rules = default_rules();
    rules.name = "high_factory".to_string();
    rules.resources[1].production = |q| if q >= 4 { 45. * q as f64 } else { 12. * q as f64 };
    rules
}

pub fn named(name: &str) -> Option<GameRules> {
    match name {
        "default" => Some(default_rules()),
        "tiny" => Some(tiny_rules()),
        "high_factory" => Some(high_factory_rules()),
        _ => None,
    }
}

pub fn names() -> &'static [&'static str] {
    &["default", "tiny", "high_factory"]
}
