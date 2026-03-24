struct Resource{F<:Function,G<:Function}
    name::String
    cost_fn::F   # Function: f(x) -> cost of the next unit
    yield_fn::G  # Function: g(x) -> total production per second
end

const RESOURCES = [
    Resource("Clicker", q -> 10 * 1.1^q, q -> 2.0 * q),
    Resource("Factory", q -> 100 * 1.2^q, q -> q >= 5 ? (30.0 * q) : (10.0 * q))
]

Base.@kwdef mutable struct GameState
    time::Int64 = 0
    money::Float64 = 0.0
    inventory::Vector{Int} = zeros(size(RESOURCES))
end

get_income(state::GameState) = sum(resource.yield_fn(q) for (resource, q) in zip(RESOURCES, state.inventory))
get_cost(idx::Int, quantity::Int) = RESOURCES[idx].cost_fn(quantity)
can_afford(state, idx) = state.money >= get_cost(idx, state.inventory[idx])
get_stats(state::GameState) = println("Time: $(state.time) | Money: $(round(state.money, digits=2)) | Income: $(get_income(state))")

function step!(state::GameState)
    state.money += get_income(state)
    state.time += 1
    iszero(state.time % 100) && get_stats(state)
end


function buy!(state::GameState, idx::Int)
    if !checkbounds(Bool, RESOURCES, idx)
        return false
    end

    can_afford(state, idx) || return false  # Return early if we can't afford it
    state.money -= get_cost(idx, state.inventory[idx])
    state.inventory[idx] += 1
    return true
end


function main()
    game = GameState()

    println("Starting simulation...")

    game.inventory[1] = 1

    for s in 1:500
        step!(game)
        iszero((s + 1) % 150) && buy!(game, 2)
    end

    println("Final Wealth: ", game.money)
end

@time main()