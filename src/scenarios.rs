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
pub fn cookie_clicker_rules() -> GameRules {
    GameRules {
        name: "cookie_clicker".to_string(),
        initial_money: 20.0,
        initial_inventory: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        resources: vec![
            ResourceRule {
                name: "Cursor".to_string(),
                cost: |q| 15. * 1.15_f64.powi(q),
                production: |q| 0.1 * q as f64,
            },
            ResourceRule {
                name: "Grandma".to_string(),
                cost: |q| 100. * 1.15_f64.powi(q),
                production: |q| 1. * q as f64,
            },
            ResourceRule {
                name: "Farm".to_string(),
                cost: |q| 1100. * 1.15_f64.powi(q),
                production: |q| 8. * q as f64,
            },
            ResourceRule {
                name: "Mine".to_string(),
                cost: |q| 12000. * 1.15_f64.powi(q),
                production: |q| 47. * q as f64,
            },
            ResourceRule {
                name: "Factory".to_string(),
                cost: |q| 130_000. * 1.15_f64.powi(q),
                production: |q| 260. * q as f64,
            },
            ResourceRule {
                name: "Bank".to_string(),
                cost: |q| 1_400_000. * 1.15_f64.powi(q),
                production: |q| 1400. * q as f64,
            },
            ResourceRule {
                name: "Temple".to_string(),
                cost: |q| 20_000_000. * 1.15_f64.powi(q),
                production: |q| 7800. * q as f64,
            },
            ResourceRule {
                name: "Wizard Tower".to_string(),
                cost: |q| 330_000_000. * 1.15_f64.powi(q),
                production: |q| 44_000. * q as f64,
            },
            ResourceRule {
                name: "Shipment".to_string(),
                cost: |q| 5_100_000_000. * 1.15_f64.powi(q),
                production: |q| 260_000. * q as f64,
            },
            ResourceRule {
                name: "Alchemy Lab".to_string(),
                cost: |q| 75_000_000_000. * 1.15_f64.powi(q),
                production: |q| 1_600_000. * q as f64,
            },
            ResourceRule {
                name: "Portal".to_string(),
                cost: |q| 1_000_000_000_000. * 1.15_f64.powi(q),
                production: |q| 10_000_000. * q as f64,
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
        "cookie_clicker" => Some(cookie_clicker_rules()),
        _ => None,
    }
}

pub fn names() -> &'static [&'static str] {
    &["default", "tiny", "high_factory", "cookie_clicker"]
}
