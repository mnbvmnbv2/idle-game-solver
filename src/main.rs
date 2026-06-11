use std::{env, process, time::Instant};

use idle_game_solver::benchmark::{benchmark_suite, run_benchmarks};
use idle_game_solver::scenarios::{named, names};
use idle_game_solver::solver::{BranchAndBoundSolver, SearchConfig, SolverAlgorithm};
use idle_game_solver::tracing::{JsonTrace, NullTrace};

fn print_usage() {
    eprintln!(
        "Usage:\n  \
         cargo run --release -- solve [scenario] [goal] [trace.json]\n  \
         cargo run --release -- benchmark\n\n  \
         Scenarios: {}\n\n  \
         Backwards-compatible:\n  \
         cargo run --release -- [goal] [trace.json]",
        names().join(", ")
    );
}

fn solve_command(args: &[String]) {
    let scenario_name = args.get(0).map(String::as_str).unwrap_or("default");
    let goal = args.get(1).and_then(|s| s.parse::<f64>().ok());
    let trace_path = args.get(2).map(String::as_str);

    let Some(rules) = named(scenario_name, goal) else {
        eprintln!("Unknown scenario: {scenario_name}");
        print_usage();
        process::exit(2);
    };

    run_solve(rules, trace_path);
}

fn run_solve(rules: idle_game_solver::game::GameRules, trace_path: Option<&str>) {
    let solver = BranchAndBoundSolver::new(SearchConfig::default());
    let start = Instant::now();

    if let Some(path) = trace_path {
        let mut trace = JsonTrace::new(&rules);
        let result = solver.solve(rules, &mut trace);
        println!("Time elapsed: {:?}\n{result:?}", start.elapsed());

        match trace.write(path, &result) {
            Ok(()) => println!("Wrote trace to {path}"),
            Err(e) => eprintln!("Failed to write trace: {e}"),
        }
    } else {
        let mut trace = NullTrace::default();
        let result = solver.solve(rules, &mut trace);
        println!("Time elapsed: {:?}\n{result:?}", start.elapsed());
    }
}

fn benchmark_command() {
    let solver = BranchAndBoundSolver::new(SearchConfig::default());
    let results = run_benchmarks(&solver, &benchmark_suite());

    println!(
        "{:<25} {:<15} {:<10} {:<12} {:<12} {:<10} {:<6}",
        "Benchmark", "Solver", "Best Time", "Iterations", "Elapsed (ms)", "Expected", "Passed"
    );
    println!("{}", "-".repeat(95)); // Separator line

    for r in results {
        let expected = r.expected_best_time.map_or_else(|| "-".to_string(), |v| v.to_string());
        let passed_str = if r.passed { "YES" } else { "NO" };
        println!(
            "{:<25} {:<15} {:<10} {:<12} {:<12} {:<10} {:<6}",
            r.name,
            r.solver,
            r.result.best_time,
            r.result.iterations,
            r.elapsed.as_millis(),
            expected,
            passed_str
        );
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("solve") => solve_command(&args[1..]),
        Some("benchmark") => benchmark_command(),
        Some("help") | Some("--help") | Some("-h") | _ => print_usage(),
    }
}
