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


function get_income(state::GameState)
    total_inc = 0.0
    for i in 1:length(state.inventory)
        quantity = state.inventory[i]
        total_inc += RESOURCES[i].yield_fn(quantity)
    end
    return total_inc
end

function step!(state::GameState)
    state.money += get_income(state)
    state.time += 1

    if state.time % 100 == 0
        println("Time: $(state.time) | Money: $(round(state.money, digits=2)) | Income: $(get_income(state))")
    end
end

function main()
    game = GameState()

    println("Starting simulation...")

    game.inventory[1] = 1

    for _ in 1:500
        step!(game)
    end

    println("Final Wealth: ", game.money)
end

@time main()