use std::time::{Duration, Instant};

use crate::{
    game::GameRules,
    scenarios,
    solver::{BranchAndBoundSolver, SolveResult, SolverAlgorithm},
    tracing::NullTrace,
};

#[derive(Clone, Debug)]
pub struct BenchmarkCase {
    pub name: &'static str,
    pub rules: GameRules,
    /// Fill this in once you want strict regression detection for a case.
    pub expected_best_time: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub name: &'static str,
    pub solver: &'static str,
    pub result: SolveResult,
    pub elapsed: Duration,
    pub expected_best_time: Option<i64>,
    pub passed: bool,
}

pub fn benchmark_suite() -> Vec<BenchmarkCase> {
    vec![
        BenchmarkCase {
            name: "tiny_goal_10k",
            rules: scenarios::tiny_rules(10_000.0),
            expected_best_time: Some(137),
        },
        BenchmarkCase {
            name: "default_goal_1m",
            rules: scenarios::rules_with_goal(1_000_000.0),
            expected_best_time: Some(383),
        },
        BenchmarkCase {
            name: "default_goal_1e10",
            rules: scenarios::rules_with_goal(1e10),
            expected_best_time: Some(903476),
        },
        BenchmarkCase {
            name: "default_goal_1e12",
            rules: scenarios::rules_with_goal(1e12),
            expected_best_time: Some(65254751),
        },
        BenchmarkCase {
            name: "high_factory_goal_1m",
            rules: scenarios::high_factory_rules(1_000_000.0),
            expected_best_time: Some(347),
        },
    ]
}

pub fn run_benchmarks(
    solver: &BranchAndBoundSolver,
    cases: &[BenchmarkCase],
) -> Vec<BenchmarkResult> {
    cases
        .iter()
        .map(|case| {
            let mut trace = NullTrace::default();
            let start = Instant::now();
            let result = solver.solve(case.rules.clone(), &mut trace);
            let elapsed = start.elapsed();
            let passed =
                case.expected_best_time.map_or(true, |expected| expected == result.best_time);

            BenchmarkResult {
                name: case.name,
                solver: solver.name(),
                result,
                elapsed,
                expected_best_time: case.expected_best_time,
                passed,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_expected_best_times_stay_stable() {
        let solver = BranchAndBoundSolver::default();
        let failures: Vec<_> =
            run_benchmarks(&solver, &benchmark_suite()).into_iter().filter(|r| !r.passed).collect();

        assert!(failures.is_empty(), "benchmark regressions: {failures:#?}");
    }
}
