# idle-game-solver

A small idle-game solver with the game model separated from solver/search logic.

## Structure

- `src/game.rs` defines game rules, resource cost/production curves, state, actions, and action application.
- `src/scenarios.rs` defines named rule sets. Add new resources or changed values here.
- `src/solver.rs` contains the current branch-and-bound solver and the `SolverAlgorithm` trait for future solvers.
- `src/benchmark.rs` defines a benchmark/regression suite over multiple rule sets.
- `src/tracing.rs` observes solver events and writes `trace.json` for `visualizer.html`.

## Run

```bash
cargo run --release -- solve default 1000000 trace.json
cargo run --release -- solve tiny 10000
cargo run --release -- benchmark
```

## Add or change game rules

Edit `src/scenarios.rs` and return a `GameRules` value:

```rust
GameRules {
    name: "my_game".to_string(),
    goal,
    initial_money: 0.0,
    initial_inventory: vec![1, 0, 0],
    resources: vec![/* ResourceRule values */],
}
```

## Regression checks

`benchmark_suite()` includes several cases with expected `best_time` values. Run:

```bash
cargo test
cargo run --release -- benchmark
```

When intentionally changing solver behavior or game rules, update the expected values in `src/benchmark.rs` after reviewing the new results.
