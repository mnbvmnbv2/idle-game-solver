use std::{env, process, time::Instant};

use idle_game_solver::benchmark::{benchmark_suite, run_benchmarks};
use idle_game_solver::game::GameRules;
use idle_game_solver::objective::Objective;
use idle_game_solver::scenarios::{named, names};
use idle_game_solver::solver::{BranchAndBoundSolver, SearchConfig, SolverAlgorithm};
use idle_game_solver::tracing::{JsonTrace, NullTrace};

fn print_usage() {
    eprintln!(
        "Usage:\n  \
         cargo run --release -- solve [scenario] [objective] [trace.json]\n  \
         cargo run --release -- solve default money:1000000 trace.json\n  \
         cargo run --release -- solve default inventory:2,10,5 trace.json\n  \
         cargo run --release -- solve default income:10000 trace.json\n  \
         cargo run --release -- benchmark\n\n  \
         Objectives: money:<amount>, inventory:<q0,q1,...>, income:<amount>\n  \
         A bare number is treated as money:<amount>.\n  \
         Scenarios: {}\n\n  \
         Backwards-compatible:\n  \
         cargo run --release -- [goal] [trace.json]",
        names().join(", ")
    );
}

fn parse_inventory(raw: &str) -> Result<Vec<i32>, String> {
    raw.split(',')
        .map(|part| {
            part.trim().parse::<i32>().map_err(|_| format!("Invalid inventory quantity: {part}"))
        })
        .collect()
}

fn parse_objective(raw: &str) -> Result<Objective, String> {
    if let Some(amount) = raw.strip_prefix("money:") {
        return amount
            .parse::<f64>()
            .map(Objective::money)
            .map_err(|_| format!("Invalid money objective: {raw}"));
    }

    if let Some(quantities) = raw.strip_prefix("inventory:") {
        return parse_inventory(quantities).map(Objective::inventory_at_least);
    }

    if let Some(amount) = raw.strip_prefix("income:") {
        return amount
            .parse::<f64>()
            .map(Objective::income_at_least)
            .map_err(|_| format!("Invalid income objective: {raw}"));
    }

    raw.parse::<f64>().map(Objective::money).map_err(|_| format!("Invalid objective: {raw}"))
}

fn solve_command(args: &[String]) {
    let first = args.get(0).map(String::as_str);
    let first_is_scenario = first.map_or(false, |value| names().contains(&value));

    let scenario_name = if first_is_scenario { first.unwrap() } else { "default" };
    let objective_arg = args.get(if first_is_scenario { 1 } else { 0 }).map(String::as_str);
    let trace_path = args.get(if first_is_scenario { 2 } else { 1 }).map(String::as_str);

    let parsed_objective = match objective_arg.map(parse_objective).transpose() {
        Ok(objective) => objective,
        Err(e) => {
            eprintln!("{e}");
            print_usage();
            process::exit(2);
        }
    };

    let Some(rules) = named(scenario_name) else {
        eprintln!("Unknown scenario: {scenario_name}");
        print_usage();
        process::exit(2);
    };

    let objective = parsed_objective.unwrap_or_else(|| Objective::money(1e12));
    run_solve(rules, objective, trace_path);
}

fn run_solve(rules: GameRules, objective: Objective, trace_path: Option<&str>) {
    let solver = BranchAndBoundSolver::new(SearchConfig::default());
    let start = Instant::now();

    if let Some(path) = trace_path {
        let mut trace = JsonTrace::new(&rules, &objective);
        let result = solver.solve(rules, objective, &mut trace);
        println!("Time elapsed: {:?}\n{result:?}", start.elapsed());

        match trace.write(path, &result) {
            Ok(()) => println!("Wrote trace to {path}"),
            Err(e) => eprintln!("Failed to write trace: {e}"),
        }
    } else {
        let mut trace = NullTrace::default();
        let result = solver.solve(rules, objective, &mut trace);
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
    println!("{}", "-".repeat(95));

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
        Some("help") | Some("--help") | Some("-h") | None => print_usage(),
        Some(_) => solve_command(&args),
    }
}
