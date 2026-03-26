using DataStructures

# --- base structs ---
struct Resource{F<:Function,G<:Function}
    name::String
    cost_fn::F   # Function: f(x) -> cost of the next unit
    yield_fn::G  # Function: g(x) -> total production per second
end

const RESOURCES = (
    Resource("Clicker", q -> 10 * 1.1^q, q -> 2.0 * q),
    Resource("Factory", q -> 100 * 1.2^q, q -> q >= 5 ? (30.0 * q) : (10.0 * q))
)

struct History
    action_idx::Int
    time::Int
    prev::Union{Nothing,History}
end

struct GameState{N}
    time::Int64
    money::Float64
    inventory::NTuple{N,Int}
    history::Union{Nothing,History}
end

GameState() = GameState(0, 0.0, (1, 0), nothing)

# --- transitions and helpers ---

get_income(state::GameState) = sum(resource.yield_fn(q) for (resource, q) in zip(RESOURCES, state.inventory))
get_cost(idx::Int, s::GameState) = RESOURCES[idx].cost_fn(s.inventory[idx])
can_afford(state, idx) = state.money >= get_cost(idx, state)
get_stats(state::GameState) = println("Time: $(state.time) | Money: $(round(state.money, digits=2)) | Income: $(get_income(state))")

function step(s::GameState, ticks::Int=1)
    new_money = s.money + (get_income(s) * ticks)
    return GameState(s.time + ticks, new_money, s.inventory, s.history)
end

function buy(s::GameState, idx::Int)
    price = get_cost(idx, s)

    s.money < price && return nothing

    new_inv = ntuple(i -> i == idx ? s.inventory[i] + 1 : s.inventory[i], length(s.inventory))

    return GameState(s.time, s.money - price, new_inv, History(idx, s.time, s.history))
end

# --- solver stuff ---

function time_to_money(s::GameState, money::Float64)::Int
    income = get_income(s)
    income <= 0.0 && return typemax(Int)

    remaining = max(0.0, money - s.money)
    return ceil(Int, remaining / income)
end

function buy_order(s::GameState, order::Int, goal::Float64)
    time_to_goal = time_to_money(s, goal)
    time_to_resource = time_to_money(s, get_cost(order, s))
    if time_to_goal < time_to_resource
        return (game=s, time=time_to_goal)
    else
        s = step(s, time_to_resource)
        s = buy(s, order)
    end
    return (game=s, time=time_to_money(s, goal))
end

function get_history(s::GameState)
    log = []
    curr = s.history
    while !isnothing(curr)
        push!(log, "T$(curr.time): $(RESOURCES[curr.action_idx].name)")
        curr = curr.prev
    end
    return reverse(log)
end

function main(goal::Float64=1e10)
    game = GameState()

    queue = Deque{Tuple{GameState{2},Int64}}()
    best = Inf
    best_game = nothing

    for iter in 1:1_000_000
        for idx in 1:length(RESOURCES)
            push!(queue, (game, idx))
        end
        game, order = popfirst!(queue)
        game, time = buy_order(game, order, goal)
        if game.time + time < best
            best = game.time + time
            best_game = game
        end
    end

    # finish best game
    best_game = step(best_game, time_to_money(best_game, goal))

    println("Best history $(get_history(best_game))")

    println("Final Wealth: $(best_game.money) in $(best)")
end

@time main()