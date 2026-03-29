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

struct GameState{N}
    time::Int64
    money::Float64
    inventory::NTuple{N,Int}
end

GameState() = GameState(0, 0.0, (1, 0))

# --- transitions and helpers ---

get_income(state::GameState) = sum(resource.yield_fn(q) for (resource, q) in zip(RESOURCES, state.inventory))
get_cost(idx::Int, s::GameState) = RESOURCES[idx].cost_fn(s.inventory[idx])
can_afford(state, idx) = state.money >= get_cost(idx, state)
get_stats(state::GameState) = println("Time: $(state.time) | Money: $(round(state.money, digits=2)) | Income: $(get_income(state))")

function step(s::GameState, ticks::Int=1)
    return GameState(s.time + ticks, s.money + (get_income(s) * ticks), s.inventory)
end

function buy(s::GameState, idx::Int)
    price = get_cost(idx, s)

    s.money < price && return nothing

    new_inv = ntuple(i -> i == idx ? s.inventory[i] + 1 : s.inventory[i], length(s.inventory))

    return GameState(s.time, s.money - price, new_inv)
end

# --- solver stuff ---

function time_to_money(s::GameState, money::Float64)::Int
    income = get_income(s)
    income <= 0.0 && return typemax(Int)

    return ceil(Int, max(0.0, money - s.money) / income)
end

function buy_order(s::GameState, order::Int, goal::Float64)
    time_to_goal = time_to_money(s, goal)
    time_to_resource = time_to_money(s, get_cost(order, s))
    if time_to_goal <= time_to_resource
        return (game=s, time=s.time + time_to_goal, done=true)
    end
    s = buy(step(s, time_to_resource), order)
    return (game=s, time=s.time + time_to_money(s, goal), done=false)
end

function reconstruct_path(memory, final_inventory)
    log = String[]
    curr_inv = final_inventory

    while true
        mem_entry = get(memory, curr_inv, nothing)
        isnothing(mem_entry) && break

        time, money, parent_inv, action = mem_entry
        parent_inv = ntuple(i -> i == action ? parent_inv[i] - 1 : parent_inv[i], length(parent_inv))

        action == 0 && break

        push!(log, "Reached $curr_inv by buying $(RESOURCES[action].name) at time $time")

        curr_inv = parent_inv
    end
    return reverse(log)
end


function bfs(goal::Float64=1e10)
    next_game = GameState()
    memory = Dict{NTuple{2,Int},Tuple{Int64,Float64,NTuple{2,Int},Int}}()

    queue = Deque{Tuple{GameState{2},Int64}}()
    for idx in 1:length(RESOURCES)
        push!(queue, (next_game, idx))
    end
    best_finish_time = next_game.time + time_to_money(next_game, goal)
    best_game = next_game

    iter = 0
    while !isempty(queue) && iter < 1_000_000
        iter += 1
        curr_game, order = popfirst!(queue)

        next_game, time, done = buy_order(curr_game, order, goal)

        is_worse_than_parent = time >= curr_game.time + time_to_money(curr_game, goal)
        is_worse_than_parent && continue

        best_mem_time, best_mem_money = get(memory, next_game.inventory, (typemax(Int64), 0.0))
        is_better_than_memory = (next_game.time < best_mem_time) ||
                                (next_game.time == best_mem_time && next_game.money > best_mem_money)
        if is_better_than_memory
            memory[next_game.inventory] = (next_game.time, next_game.money, next_game.inventory, order)
            if !done && next_game.time < best_finish_time
                for idx in 1:length(RESOURCES)
                    push!(queue, (next_game, idx))
                end
            end
        end

        if time < best_finish_time
            best_finish_time = time
            best_game = next_game
        end
    end

    # finish best game
    best_game = step(best_game, time_to_money(best_game, goal))

    println("\nBest history:\n", join(reconstruct_path(memory, best_game.inventory), "\n"))

    println("Final Wealth: $(best_game.money) in $(best_finish_time)")
end

struct QueueNode{N}
    priority::Int
    game::GameState{N}
    order::Int
end
Base.isless(a::QueueNode, b::QueueNode) = a.priority < b.priority

function best_first(goal::Float64=1e15)
    start_game = GameState()

    memory = Dict{NTuple{2,Int},Tuple{Int64,Float64,NTuple{2,Int},Int}}()
    memory[start_game.inventory] = (start_game.time, start_game.money, start_game.inventory, 0)

    best_finish_time = start_game.time + time_to_money(start_game, goal)
    best_game = start_game

    pq = BinaryMinHeap{QueueNode{2}}()
    for idx in 1:length(RESOURCES)
        push!(pq, QueueNode(start_game.time, start_game, idx))
    end

    iter = 0
    while !isempty(pq) && iter < 1_000_000
        iter += 1
        node = pop!(pq)
        curr_game = node.game
        order = node.order

        next_game, finish_time, done = buy_order(curr_game, order, goal)

        is_worse_than_parent = finish_time >= curr_game.time + time_to_money(curr_game, goal)
        is_worse_than_parent && continue

        mem_entry = get(memory, next_game.inventory, (typemax(Int64), 0.0, (0, 0), 0))
        best_mem_time = mem_entry[1]
        best_mem_money = mem_entry[2]
        is_better_than_memory = (next_game.time < best_mem_time) ||
                                (next_game.time == best_mem_time && next_game.money > best_mem_money)
        if is_better_than_memory
            memory[next_game.inventory] = (next_game.time, next_game.money, next_game.inventory, order)
            if !done && next_game.time < best_finish_time
                for idx in 1:length(RESOURCES)
                    push!(pq, QueueNode(next_game.time, next_game, idx))
                end
            end
        end

        if finish_time < best_finish_time
            best_finish_time = finish_time
            best_game = next_game
            println("Iter: $(iter): New Best Time Found: $best_finish_time")
        end
    end

    # finish best game
    best_game = step(best_game, time_to_money(best_game, goal))

    println("\nBest history:\n", join(reconstruct_path(memory, best_game.inventory), "\n"))

    println("Final Wealth: $(best_game.money) in $(best_finish_time)")

    println("Did $(iter) iterations")
end

@time best_first()