# --- base structs ---
struct Resource{F<:Function,G<:Function}
    name::String
    cost_fn::F   # Function: f(x) -> cost of the next unit
    yield_fn::G  # Function: g(x) -> total production per second
end

const RESOURCES = [
    Resource("Clicker", q -> 10 * 1.1^q, q -> 2.0 * q),
    Resource("Factory", q -> 100 * 1.2^q, q -> q >= 5 ? (30.0 * q) : (10.0 * q))
]

struct GameState{N}
    time::Int64
    money::Float64
    inventory::NTuple{N,Int}
end

GameState() = GameState(0, 0.0, (1, 0))

# --- transitions and helpers ---

get_income(state::GameState) = sum(resource.yield_fn(q) for (resource, q) in zip(RESOURCES, state.inventory))
get_cost(idx::Int, quantity::Int) = RESOURCES[idx].cost_fn(quantity)
can_afford(state, idx) = state.money >= get_cost(idx, state.inventory[idx])
get_stats(state::GameState) = println("Time: $(state.time) | Money: $(round(state.money, digits=2)) | Income: $(get_income(state))")

function step(s::GameState, ticks::Int=1)
    new_money = s.money + (get_income(s) * ticks)
    return GameState(s.time + ticks, new_money, s.inventory)
end

function buy(s::GameState, idx::Int)
    checkbounds(Bool, RESOURCES, idx) || return nothing

    price = get_cost(idx, s.inventory[idx])

    s.money < price && return nothing

    new_inv = ntuple(i -> i == idx ? s.inventory[i] + 1 : s.inventory[i], length(s.inventory))

    return GameState(s.time, s.money - price, new_inv)
end

# --- solver stuff ---

function time_to_goal(s::GameState, goal::Float64)
    income = get_income(s)
    income <= 0.0 && return Inf

    remaining = max(0.0, goal - s.money)
    return ceil(Int, remaining / income)
end

function main(goal::Float64=1e10)
    game = GameState()

    to_goal = Inf
    s = 0
    while to_goal == Inf
        s += 1
        ttg = time_to_goal(game, goal)
        for r in 1:length(RESOURCES)
            possible_game = buy(game, r)
            if isnothing(possible_game)
                continue
            end

            new_to_goal = time_to_goal(possible_game, goal)
            if new_to_goal < ttg
                game = possible_game
                ttg = new_to_goal
            end
        end
        game = step(game)
        if game.money >= goal
            to_goal = s
            break
        end
    end

    println("Final Wealth: $(game.money) in $(to_goal)")
end

@time main()