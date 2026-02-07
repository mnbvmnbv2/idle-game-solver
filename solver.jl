include("game_logic.jl")

abstract type AbstractGoal end

struct ReachMoney <: AbstractGoal
    target::Float64
end

struct ReachYield <: AbstractGoal
    target::Float64
end

struct ReachWealth <: AbstractGoal
    target::Float64 # Total Value (Cash + Cost of buildings owned)
end

function seconds_to_afford(current_money, income, cost)
    if current_money >= cost
        return 0.0
    elseif income <= 0
        return Inf # Never (infinity)
    else
        return (cost - current_money) / income
    end
end

function evaluate_best_move(state::GameState, resources, goal::ReachMoney)
    best_time = seconds_to_afford(state.money, state.income, goal.target)
    best_action_idx = 0 # 0 means "Wait until goal reached"

    for (i, res) in enumerate(resources)
        qty = state.inventory[i]
        cost = res.cost_fn(qty)

        t_buy = seconds_to_afford(state.money, state.income, cost)

        added_yield = res.yield_fn(qty + 1) - res.yield_fn(qty)
        new_income = state.income + added_yield

        remaining_money_needed = goal.target

        t_after = seconds_to_afford(0.0, new_income, goal.target)

        total_time = t_buy + t_after

        if total_time < best_time
            best_time = total_time
            best_action_idx = i
        end
    end
    return best_action_idx
end

function evaluate_best_move(state::GameState, resources, goal::ReachYield)
    if state.income >= goal.target
        return 0 # Done!
    end

    best_score = Inf
    best_action_idx = 0

    for (i, res) in enumerate(resources)
        qty = state.inventory[i]
        cost = res.cost_fn(qty)
        gain = res.yield_fn(qty + 1) - res.yield_fn(qty)

        if gain <= 0
            continue
        end

        time_to_buy = seconds_to_afford(state.money, state.income, cost)

        score = time_to_buy / gain

        if score < best_score
            best_score = score
            best_action_idx = i
        end
    end
    return best_action_idx
end

function solve(resources, goal::AbstractGoal)
    state = GameState(
        time=0.0,
        money=100.0,
        income=1.0, # Base income
        inventory=zeros(Int, length(resources))
    )

    history = []

    println("--- SOLVING FOR: $(typeof(goal)) $(goal.target) ---")

    while true
        if (goal isa ReachMoney && state.money >= goal.target) ||
           (goal isa ReachYield && state.income >= goal.target)
            println("Goal Reached at t=$(round(state.time, digits=2))s")
            break
        end

        idx = evaluate_best_move(state, resources, goal)

        if idx == 0
            if goal isa ReachMoney
                t_left = seconds_to_afford(state.money, state.income, goal.target)
                state = GameState(state.time + t_left, goal.target, state.income, state.inventory)
                push!(history, (:finish, t_left))
            else
                break
            end
        else
            res = resources[idx]
            qty = state.inventory[idx]
            cost = res.cost_fn(qty)

            dt = seconds_to_afford(state.money, state.income, cost)

            new_money = (state.money + (state.income * dt)) - cost
            new_inv = copy(state.inventory)
            new_inv[idx] += 1

            added_yield = res.yield_fn(qty + 1) - res.yield_fn(qty)

            state = GameState(
                state.time + dt,
                new_money,
                state.income + added_yield,
                new_inv
            )

            push!(history, (:buy, res.name, state.time))
            println("Bought $(res.name) at t=$(round(state.time, digits=2))")
        end
    end
end

r1 = Resource("Worker", x -> 10 * 1.1^x, x -> 1 * x)
r2 = Resource("Machine", x -> 100 * 1.5^x, x -> 10 * x)

resources = [r1, r2]

solve(resources, ReachMoney(1000.0))

println("\n--- NEW GOAL ---\n")

solve(resources, ReachYield(50.0))